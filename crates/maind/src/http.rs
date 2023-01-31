//! Run with
//!
//! ```not_rust
//! cd examples && cargo run -p example-static-file-server
//! ```

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get_service, post},
    Json, Router,
};
use itertools::Itertools;
use log::info;
use serde::Deserialize;
use std::{io, io::BufRead, net::SocketAddr};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

pub async fn httpd() {
    let serve_dir =
        get_service(ServeDir::new("web/build").fallback(ServeFile::new("web/build/index.html")))
            .handle_error(handle_error);
    let app = Router::new()
        .nest_service("/", serve_dir)
        .route("/api/tesla/track", post(track));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3600));
    info!("listening on asset {}", addr);
    axum::Server::bind(&addr)
        .serve(app.layer(TraceLayer::new_for_http()).into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, serde::Serialize, Deserialize)]
struct ReqTrackData {}
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct VehiclePosition {
    pub longitude: f64,
    pub latitude: f64,
    pub elevation: f64,
}
#[derive(Debug, Default, serde::Serialize, Deserialize)]
struct RspTrackData {
    footprint: Vec<VehiclePosition>,
}

const X_PI: f64 = std::f64::consts::PI * 3000.0 / 180.0;
fn gcj02_to_bd09(gcj_lat: f64, gcj_lng: f64) -> (f64, f64) {
    let z = (gcj_lng * gcj_lng + gcj_lat * gcj_lat).sqrt() + 0.00002 * (gcj_lat * X_PI).sin();
    let theta = gcj_lat.atan2(gcj_lng) + 0.000003 * (gcj_lng * X_PI).cos();
    (z * theta.sin() + 0.006, z * theta.cos() + 0.0065)
}

/// return the track data from append.log
async fn track() -> Json<RspTrackData> {
    let mut rsp = RspTrackData::default();
    let f = std::fs::File::open(".cache/logs/2023_01_30.log");
    if let Ok(file) = f {
        let lines = std::io::BufReader::new(file)
            .lines()
            .filter(|l| l.is_ok() && l.as_ref().unwrap().contains("timestamp"))
            .map(|l| {
                let update: tesla_api::StreamDataUpdate =
                    serde_json::from_str(&l.as_ref().unwrap()).unwrap();
                update
            })
            .collect_vec();
        rsp.footprint = lines
            .iter()
            // .take(10000)
            .map(|update| {
                let (lat, lng) = eviltransform::wgs2gcj(update.est_lat, update.est_lng);
                let (lat, lng) = gcj02_to_bd09(lat, lng);
                // let (lat, lng) = (update.est_lat, update.est_lng);
                // panic!("update={:?} lat={lat} lng={lng}", update);
                let pos = VehiclePosition {
                    longitude: lng,
                    latitude: lat,
                    elevation: update.elevation,
                };
                pos
            })
            .collect_vec();
    }
    Json(rsp)
}

async fn handle_error(_err: io::Error) -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
}
