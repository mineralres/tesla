use log::{error, info};
use tesla_api::{ApiClient, ApiError, VehicleData};

mod http;
use http::*;

fn init_logger() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::init();
}

#[tokio::main]
async fn main() {
    init_logger();
    check_make_dir(".cache");
    check_make_dir(".cache/logs");
    tokio::spawn(async {
        httpd().await;
    });
    let cookie = r#"gdp_user_id=gioenc-c5d09234,8ccd,5bd9,a37d,5e54ceaed440;"#;
    let api_root = "https://owner-api.vn.cloud.tesla.cn";
    // let stream_path = "wss://streaming.vn.teslamotors.com/streaming/";
    let stream_path = "wss://streaming.vn.cloud.tesla.cn/streaming/";
    let mut client = ApiClient::init(&cookie, api_root, stream_path).await;

    let user = client.users_me().await;
    match user {
        Ok(user) => {
            info!("user={:?}", user);
        }
        Err(e) => match e {
            ApiError::Unauthorized => {
                info!("Get new access token");
                let _result = client
                    .get_token("", "")
                    .await
                    .expect("Failed to get new access_token");
            }
            _ => error!("api err={}", e),
        },
    }
    let vehicles = client.vehicles().await.expect("");
    info!("vehicles={:?}", vehicles);
    let vehicle_data = client.vehicle_data(vehicles[0].id).await;
    info!("vehicle_data={:?}", vehicle_data);
    if let Ok(d) = vehicle_data {
        save_vehicle_data(&d)
            .await
            .expect("save vehicle data failed");
    }
    loop {
        let mut client = ApiClient::init(&cookie, api_root, stream_path).await;
        info!("Create new ApiClient");
        loop {
            info!("Start streaming");
            let resp = client.stream(vehicles[0].vehicle_id).await;
            if let Err(e) = resp {
                match e {
                    ApiError::Unauthorized => {
                        info!("Stream unauthorized, get new access token");
                        let _result = client
                            .get_token("", "")
                            .await
                            .expect("Failed to get new access_token");
                    }
                    _ => {
                        error!("stream err={}", e)
                    }
                }
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }
}

pub async fn save_vehicle_data(d: &VehicleData) -> Result<(), std::io::Error> {
    let p = format!(".cache/vehicle_data_{}.json", d.id);
    std::fs::write(&p, serde_json::to_string_pretty(d).unwrap())?;
    Ok(())
}

pub fn check_make_dir(dir: &str) {
    match std::fs::create_dir_all(dir) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
            } else {
                panic!("create dir={} err={}", dir, e);
            }
        }
    }
}
