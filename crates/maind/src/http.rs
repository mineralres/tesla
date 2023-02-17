//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-static-file-server
//! ```

use pb::tesla::*;

use crate::config::*;
use crate::local_cache;
use axum::{
    extract::{Host, Json, State},
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::{get_service, post},
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use derive_more::{Display, From};
use log::info;
use prost::Message;
use serde::Deserialize;
use std::sync::Arc;
use std::{io, net::SocketAddr, path::PathBuf};
use tesla_api::*;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

type TeslaApiClient = Arc<Mutex<ApiClient>>;

#[derive(Debug, Display, From)]
pub enum HttpError {
    ApiError(ApiError),
    StdIoError(std::io::Error),
}
impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let body = match self {
            HttpError::ApiError(e) => format!("api error {}", e),
            HttpError::StdIoError(e) => format!("std io error {}", e),
        };
        // its often easiest to implement `IntoResponse` by calling other implementations
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

pub async fn httpd(api_client: ApiClient, conf: &Config) {
    let state = Arc::new(Mutex::new(api_client));
    let s2 = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            // refresh token / 5 hours
            tokio::time::sleep(tokio::time::Duration::from_secs(3600 * 5)).await;
            let mut s2 = s2.lock().await;
            info!("Refresh token for httpd tesla api client");
            s2.refresh_token().await.unwrap();
        }
    });

    let serve_dir =
        get_service(ServeDir::new("web/build").fallback(ServeFile::new("web/build/index.html")))
            .handle_error(handle_error);
    let app = Router::new()
        .nest_service("/", serve_dir)
        .route("/api/tesla/track", post(track))
        .route("/api/tesla/vehicles", post(vehicles))
        .route("/api/tesla/vehicle_data", post(vehicle_data))
        .route("/api/tesla/user_me", post(user_me))
        .route("/api/tesla/history_trips", post(history_trips))
        .route("/api/tesla/history_charges", post(history_charges))
        .with_state(state);

    let ports = Ports {
        http: conf.http_port,
        https: conf.https_port,
    };
    // optional: spawn a second server to redirect http requests to this server
    if ports.https > 0 {
        tokio::spawn(redirect_http_to_https(ports));
        // configure certificate and private key used by https
        let p1 = PathBuf::from("configs")
            .join("self_signed_certs")
            .join("cert.pem");
        info!("p1={:?}", p1);
        let config = RustlsConfig::from_pem_file(
            PathBuf::from("configs")
                .join("self_signed_certs")
                .join("cert.pem"),
            PathBuf::from("configs")
                .join("self_signed_certs")
                .join("key.pem"),
        )
        .await
        .unwrap();

        let addr = SocketAddr::from(([0, 0, 0, 0], ports.https));
        info!("listening on asset {}", addr);
        axum_server::bind_rustls(addr, config)
            .serve(app.layer(TraceLayer::new_for_http()).into_make_service())
            .await
            .unwrap();
    } else {
        let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
        info!("listening on asset {}", addr);
        axum::Server::bind(&addr)
            .serve(app.layer(TraceLayer::new_for_http()).into_make_service())
            .await
            .unwrap();
    }
}

async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
    tracing::debug!("http redirect listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(redirect.into_make_service())
        .await
        .unwrap();
}

/// track data request
#[derive(Debug, serde::Serialize, Deserialize)]
struct ReqTrackData {}

/// vehicle position
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct VehiclePosition {
    pub longitude: f64,
    pub latitude: f64,
    pub elevation: f64,
}

/// response for track
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct RspTrackData {
    longitude: Vec<f64>,
    latitude: Vec<f64>,
    elevation: Vec<f64>,
}

/// trans gcj02 to bd09 cordinates
const X_PI: f64 = std::f64::consts::PI * 3000.0 / 180.0;
fn gcj02_to_bd09(gcj_lat: f64, gcj_lng: f64) -> (f64, f64) {
    let z = (gcj_lng * gcj_lng + gcj_lat * gcj_lat).sqrt() + 0.00002 * (gcj_lat * X_PI).sin();
    let theta = gcj_lat.atan2(gcj_lng) + 0.000003 * (gcj_lng * X_PI).cos();
    (z * theta.sin() + 0.006, z * theta.cos() + 0.0065)
}

/// trans wgs to bd09
fn wgs_to_bd09(latitude: f64, longitude: f64) -> (f64, f64) {
    let (lat, lng) = eviltransform::wgs2gcj(latitude, longitude);
    gcj02_to_bd09(lat, lng)
}

#[derive(Deserialize)]
struct VehicleTrackRequest {
    id: i64,
}

/// return the track data from append.log
async fn track(
    State(_api): State<TeslaApiClient>,
    Json(req): Json<VehicleTrackRequest>,
) -> Json<RspTrackData> {
    let mut rsp = RspTrackData::default();
    match local_cache::LocalStream::load(req.id) {
        Ok(v) => {
            v.iter()
                .filter(|vd| vd.driving_state.is_some())
                .for_each(|vd| {
                    let ds = vd.driving_state.as_ref().unwrap();
                    let (lat, lng) = wgs_to_bd09(ds.est_lat, ds.est_lng);
                    rsp.longitude.push(lng);
                    rsp.latitude.push(lat);
                    rsp.elevation.push(ds.elevation);
                });
        }
        Err(_e) => (),
    }
    Json(rsp)
}

#[derive(Deserialize)]
struct HistoryTripsRequest {
    id: i64,
}

/// response for track
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct HistoryTripsResponse {
    trips: Vec<Trip>,
}

/// history trip
async fn history_trips(
    State(_api): State<TeslaApiClient>,
    Json(req): Json<HistoryTripsRequest>,
) -> Json<HistoryTripsResponse> {
    let mut rsp = HistoryTripsResponse::default();
    let mut one_trip = Trip::default();
    match local_cache::LocalStream::load(req.id) {
        Ok(v) => {
            v.iter().for_each(|vd| {
                if vd.state == "online" {
                    let mut s = TripSnapshot::default();
                    if let Some(ds) = &vd.driving_state {
                        if ds.timestamp < 1675843854 * 1000 {
                            s.timestamp = ds.timestamp * 1000;
                        } else {
                            s.timestamp = ds.timestamp;
                        }
                        let (lat, lng) = wgs_to_bd09(ds.est_lat, ds.est_lng);
                        s.latitude = lat;
                        s.longitude = lng;
                        s.elevation = ds.elevation;
                        if let Some(cs) = &vd.climate_state {
                            s.inside_temperature = cs.inside_temp;
                            s.outside_temperature = cs.outside_temp;
                        }
                    }
                    if s.timestamp > 0 {
                        one_trip.track.push(s);
                    }
                } else {
                    if one_trip.track.len() > 0 {
                        rsp.trips.push(one_trip.clone());
                        one_trip.track.clear();
                    }
                }
            });
        }
        Err(_e) => (),
    }
    Json(rsp)
}

#[derive(Deserialize)]
struct HistoryChargesRequest {
    id: i64,
}

/// response for track
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct HistoryChargesResponse {
    history_charges: Vec<HistoryCharge>,
}
/// history charge
async fn history_charges(
    State(_api): State<TeslaApiClient>,
    Json(req): Json<HistoryChargesRequest>,
) -> Json<HistoryChargesResponse> {
    let mut rsp = HistoryChargesResponse::default();
    let mut one_history_charge = HistoryCharge::default();
    match local_cache::LocalStream::load(req.id) {
        Ok(v) => {
            v.iter()
                .filter(|vd| vd.charge_state.is_some())
                .for_each(|vd| {
                    let cs = vd.charge_state.as_ref().unwrap();
                    if cs.charger_power > 0.0 {
                        one_history_charge.details.push(cs.clone());
                    } else {
                        if one_history_charge.details.len() > 0 {
                            rsp.history_charges.push(one_history_charge.clone());
                            one_history_charge.clear();
                        }
                    }
                });
        }
        Err(_e) => (),
    }
    Json(rsp)
}

/// handle error
async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}

/// get vehicles
async fn vehicles(State(api): State<TeslaApiClient>) -> Result<Json<Vec<Vehicle>>, HttpError> {
    let v = api.lock().await.vehicles().await?;
    Ok(axum::Json(v))
}

/// user me
async fn user_me(State(api): State<TeslaApiClient>) -> Result<Json<UsersMeResponse>, HttpError> {
    let u = api.lock().await.users_me().await?;
    Ok(axum::Json(u))
}

#[derive(Deserialize)]
struct VehicleDataRequest {
    id: i64,
}

/// get vehicle data
async fn vehicle_data(
    State(api): State<TeslaApiClient>,
    Json(req): Json<VehicleDataRequest>,
) -> Result<Json<VehicleData>, HttpError> {
    let mut vd = match api.lock().await.vehicle_data(req.id).await {
        Ok(vd) => vd,
        Err(_e) => {
            let path = format!(".cache/{}/vehicle_data.json", req.id);
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            let mut vd: VehicleData = serde_json::from_reader(reader).unwrap();
            vd.state = "asleep".to_string();
            vd
        }
    };
    if let Some(mut ds) = vd.drive_state.as_mut() {
        let (latitude, longitude) = wgs_to_bd09(ds.latitude, ds.longitude);
        ds.latitude = latitude;
        ds.longitude = longitude;
    }
    Ok(axum::Json(vd))
}
