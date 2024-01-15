//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-static-file-server
//! ```

use axum::{
    extract::{Json, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get_service, post},
    Router,
};
use base::{pb::base::*, pb::tesla::*};
use chrono::Local;
use db::pika::PikaConnection;
use derive_more::{Display, From};
use log::info;
use serde::Deserialize;
use std::sync::Arc;
use std::{ffi::CStr, net::SocketAddr};
use tesla_api::*;
use tokio::sync::Mutex;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Debug, Display, From)]
pub enum HttpError {
    ApiError(tesla_api::Error),
    StdIoError(std::io::Error),
    SerdeJsonErr(serde_json::Error),
    DbErr(db::Error),
}
impl axum::response::IntoResponse for HttpError {
    fn into_response(self) -> Response {
        use HttpError::*;
        let body = match self {
            ApiError(e) => format!("api error {}", e),
            StdIoError(e) => format!("std io error {}", e),
            SerdeJsonErr(e) => format!("json error {}", e),
            DbErr(e) => format!("db err:{e}"),
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

async fn my_middleware(State(_api): State<MyStateType>, request: Request, next: Next) -> Response {
    // if !api.lock().await.token.lock().await.is_valid() {
    //     let resp = Response::builder()
    //         .status(axum::http::StatusCode::SEE_OTHER)
    //         .header("location", "/set_api_token")
    //         .body(axum::body::Body::empty())
    //         .unwrap();
    //     return resp;
    // }
    let response = next.run(request).await;
    response
}

#[derive(Clone)]
struct MyStateType {
    api: Arc<Mutex<ApiClient>>,
    conf: AppConfig,
}

// type MyStateType = Arc<Mutex<MyState>>;

pub async fn httpd(api_client: ApiClient, conf: AppConfig) {
    let state = MyStateType {
        api: Arc::new(Mutex::new(api_client)),
        conf,
    };
    let ports = Ports {
        http: state.conf.http_port as u16,
        https: state.conf.https_port as u16,
    };

    let serve_dir =
        get_service(ServeDir::new("web/build").fallback(ServeFile::new("web/build/index.html")));
    let app = Router::new()
        .route("/api/tesla/track", post(track))
        .route("/api/tesla/vehicles", post(vehicles))
        .route("/api/tesla/vehicle_data", post(vehicle_data))
        .route("/api/tesla/user_me", post(user_me))
        .route("/api/tesla/history_trips", post(history_trips))
        .route("/api/tesla/history_charges", post(history_charges))
        .layer(middleware::from_fn_with_state(state.clone(), my_middleware))
        .route("/api/set_api_token", post(set_api_token))
        .nest_service("/", serve_dir)
        .with_state(state);

    // optional: spawn a second server to redirect http requests to this server
    if ports.https > 0 {
        panic!("https not supported!!!");
    } else {
        let addr = SocketAddr::from(([0, 0, 0, 0], ports.http));
        info!("listen on {addr}");
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
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

pub fn get_local_date() -> i32 {
    let local_time = Local::now();
    local_time.format("%Y%m%d").to_string().parse().unwrap()
}

/// return the track data from append.log
async fn track(
    State(s): State<MyStateType>,
    Json(req): Json<VehicleTrackRequest>,
) -> Result<Json<RspTrackData>, HttpError> {
    let mut rsp = RspTrackData::default();
    let mut pika = PikaConnection::connect(&s.conf.pika_address).await?;
    let records = pika
        .load_daily_vehicle_period_records(req.id, get_local_date())
        .await?;
    info!("records.len={}", records.len());
    for pr in records.iter() {
        for ds in pr.updates.iter() {
            let (lat, lng) = wgs_to_bd09(ds.est_lat, ds.est_lng);
            rsp.longitude.push(lng);
            rsp.latitude.push(lat);
            rsp.elevation.push(ds.elevation);
        }
    }
    Ok(Json(rsp))
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
    State(s): State<MyStateType>,
    Json(req): Json<HistoryTripsRequest>,
) -> Result<Json<HistoryTripsResponse>, HttpError> {
    let mut rsp = HistoryTripsResponse::default();
    let mut one_trip = Trip::default();
    let mut pika = PikaConnection::connect(&s.conf.pika_address).await?;
    let records = pika
        .load_daily_vehicle_period_records(req.id, get_local_date())
        .await?;
    records
        .iter()
        .filter(|r| r.snapshot.is_some())
        .for_each(|pr| {
            let snapshot = pr.snapshot.as_ref().unwrap();
            if snapshot.state == "online" {
                let mut s = TripSnapshot::default();
                for ds in pr.updates.iter() {
                    if ds.timestamp < 1675843854 * 1000 {
                        s.timestamp = ds.timestamp * 1000;
                    } else {
                        s.timestamp = ds.timestamp;
                    }
                    let (lat, lng) = wgs_to_bd09(ds.est_lat, ds.est_lng);
                    s.latitude = lat;
                    s.longitude = lng;
                    s.elevation = ds.elevation;
                    if let Some(cs) = &snapshot.climate_state {
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
    Ok(Json(rsp))
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
    State(s): State<MyStateType>,
    Json(req): Json<HistoryChargesRequest>,
) -> Result<Json<HistoryChargesResponse>, HttpError> {
    let mut rsp = HistoryChargesResponse::default();
    let mut history = HistoryCharge::default();
    let mut pika = PikaConnection::connect(&s.conf.pika_address).await?;
    let records = pika
        .load_daily_vehicle_period_records(req.id, get_local_date())
        .await?;
    for pr in records.iter() {
        if let Some(snapshot) = &pr.snapshot {
            if let Some(cs) = &snapshot.charge_state {
                if cs.charger_power > 0.0 {
                    history.details.push(cs.clone());
                }
            }
        }
    }
    // rsp.history_charges = history.details;
    Ok(Json(rsp))
}

/// response for track
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct ReqSnapshots {
    vehicle_id: i64,
    day: i32,
    forward: i32,
}

/// response for track
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct RspSnapshots {
    tt: Vec<i64>,
    charger_power: Vec<f64>,
    battery_level: Vec<f64>,
    odometer: Vec<i32>,
}

/// snapshots
async fn snapshots(
    State(s): State<MyStateType>,
    Json(req): Json<ReqSnapshots>,
) -> Result<Json<RspSnapshots>, HttpError> {
    let mut rsp = RspSnapshots::default();
    let mut pika = PikaConnection::connect(&s.conf.pika_address).await?;
    let records = pika
        .load_daily_vehicle_period_records(req.vehicle_id, get_local_date())
        .await?;
    for pr in records.iter() {
        if let Some(snapshot) = &pr.snapshot {
            if let Some(cs) = &snapshot.charge_state {
                if cs.charger_power > 0.0 {}
            }
        }
    }
    Ok(Json(rsp))
}

/// get vehicles
async fn vehicles(State(s): State<MyStateType>) -> Result<Json<Vec<Vehicle>>, HttpError> {
    let v = s.api.lock().await.vehicles().await?;
    Ok(axum::Json(v))
}

/// user me
async fn user_me(State(s): State<MyStateType>) -> Result<Json<UsersMeResponse>, HttpError> {
    let u = s.api.lock().await.users_me().await?;
    Ok(axum::Json(u))
}

#[derive(Deserialize)]
struct VehicleDataRequest {
    id: i64,
}

/// get vehicle data
async fn vehicle_data(
    State(s): State<MyStateType>,
    Json(req): Json<VehicleDataRequest>,
) -> Result<Json<VehicleData>, HttpError> {
    let mut vd = match s.api.lock().await.vehicle_data(req.id).await {
        Ok(vd) => vd,
        Err(_e) => {
            let path = format!(".cache/{}/vehicle_data.json", req.id);
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            let mut vd: VehicleData = serde_json::from_reader(reader)?;
            vd.state = "asleep".to_string();
            vd
        }
    };
    // if let Some(ds) = vd.drive_state.as_mut() {
    //     let (latitude, longitude) = wgs_to_bd09(ds.latitude, ds.longitude);
    //     ds.latitude = latitude;
    //     ds.longitude = longitude;
    // }
    Ok(axum::Json(vd))
}

#[derive(Deserialize, Debug)]
struct ReqSetApiToken {
    pub access_token: String,
    pub refresh_token: String,
}

async fn set_api_token(
    State(s): State<MyStateType>,
    Json(req): Json<ReqSetApiToken>,
) -> Result<(), HttpError> {
    info!("req={:?}", req);
    let api = s.api.lock().await;
    api.token
        .lock()
        .await
        .set_api_token(&req.access_token, &req.refresh_token)?;
    Ok(())
}
