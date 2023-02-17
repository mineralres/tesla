use clap::Parser;
use itertools::Itertools;
use log::{error, info};
use std::io::BufRead;
use tesla_api::{ApiClient, ApiError};
mod config;
mod http;
mod local_cache;
use http::*;
use local_cache::*;
use pb::tesla::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

fn init_logger() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::init();
}

#[derive(Parser)]
#[clap()]
struct Opts {
    #[clap(short, long, default_value = "")]
    email: String,
    #[clap(short, long, default_value = "")]
    password: String,
    #[clap(short, long, default_value = "configs/config.json")]
    config: String,
}

#[tokio::main]
async fn main() {
    init_logger();
    let opts: Opts = Opts::parse();
    let mut email = opts.email.clone();
    let mut password = opts.password.clone();
    if email == "" || password == "" {
        let f = std::fs::File::open(".cache/pass.txt");
        match f {
            Ok(f) => {
                let lines = std::io::BufReader::new(f)
                    .lines()
                    .map(|l| l.unwrap())
                    .collect_vec();
                email = lines[0].clone();
                password = lines[1].clone();
            }
            Err(e) => error!("{}", e),
        }
    }
    if email == "" || password == "" {
        panic!("Please set email & password in .cache/pass.txt first!");
    }

    check_make_dir(".cache");
    let conf = config::Config::load(&opts.config).expect("");
    let conf1 = conf.clone();
    let cookie = r#"gdp_user_id=gioenc-c5d09234,8ccd,5bd9,a37d,5e54ceaed440;"#;
    let mut client = ApiClient::init(&cookie, &conf.client_config).await;
    let _result = client
        .get_token(&email, &password)
        .await
        .expect("Failed to get new access_token");
    tokio::spawn(async move {
        httpd(client, &conf1).await;
    });
    let mut monitored = std::collections::HashSet::new();
    let mut client = ApiClient::init(&cookie, &conf.client_config).await;
    loop {
        let vehicles = client.vehicles().await;
        match vehicles {
            Ok(vehicles) => {
                for v in vehicles.iter() {
                    if monitored.contains(&v.id) {
                        continue;
                    }
                    monitored.insert(v.id);
                    let vehicle = (*v).clone();
                    let conf = conf.clone();
                    let conf2 = conf.clone();
                    let email = email.clone();
                    let password = password.to_string();
                    let email2 = email.clone();
                    let password2 = password.to_string();
                    let (ds_sender, mut ds_receiver) = mpsc::channel::<DrivingState>(10000);
                    let vehicle_online = Arc::new(AtomicBool::new(false));
                    let vehicle_online1 = Arc::clone(&vehicle_online);
                    tokio::spawn(async move {
                        let mut client = ApiClient::init(&cookie, &conf.client_config).await;
                        loop {
                            if !vehicle_online1.load(Ordering::Relaxed) {
                                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                                continue;
                            }
                            info!("Start {} ({}) stream.", vehicle.id, vehicle.display_name);
                            let resp = client.stream(vehicle.vehicle_id, &ds_sender).await;
                            if let Err(e) = resp {
                                match e {
                                    ApiError::Unauthorized => {
                                        info!("Stream unauthorized, get new access token");
                                        let _result = client
                                            .get_token(&email, &password)
                                            .await
                                            .expect("Failed to get new access_token");
                                    }
                                    ApiError::StreamWebSocketClosed => (),
                                    ApiError::VehicleOffline => {
                                        vehicle_online1.store(false, Ordering::Relaxed);
                                    }
                                    _ => {
                                        error!("stream err={}", e);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            1000,
                                        ))
                                        .await;
                                    }
                                }
                            }
                        }
                    });
                    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
                    tokio::spawn(async move {
                        let mut client = ApiClient::init(&cookie, &conf2.client_config).await;
                        let mut ls = local_cache::LocalStream::new(vehicle.vehicle_id);
                        let mut last_vd = VehicleData::default();
                        loop {
                            tokio::select! {
                                ds = ds_receiver.recv() => {
                                    let mut vd = last_vd.clone();
                                    vd.driving_state = ds;
                                    ls.write(&vd).expect("save vd stream failed");
                                }
                                _instant = ticker.tick() => {
                                    let vehicle_data = client.vehicle_data(vehicle.id).await;
                                    match vehicle_data {
                                        Ok(d) => {
                                            info!("vehicle state=[{}]", d.state);
                                            ls.write(&d).expect("save vd stream failed");
                                            save_vehicle_data(&d)
                                                .await
                                                .expect("save vehicle data failed");
                                            last_vd = d;
                                        }
                                        Err(e) => {
                                            match e {
                                            ApiError::Unauthorized => {
                                                info!("Stream unauthorized, get new access token");
                                                let _result = client
                                                    .get_token(&email2, &password2)
                                                    .await
                                                    .expect("Failed to get new access_token");
                                            }
                                            ApiError::VehicleOffline => {
                                                last_vd.state = "offline".to_string();
                                            }
                                            _ => {
                                               last_vd.state = "unavailable".to_string();
                                            }
                                        }
                                        error!("vehicle_data err=[{}]", e);
                                    }
                                    }
                                    ls.write(&last_vd).expect("save vd stream failed");
                                    vehicle_online.store(last_vd.state == "online", Ordering::Relaxed);
                                }
                            }
                        }
                    });
                }
            }
            Err(e) => match e {
                ApiError::Unauthorized => {
                    client.refresh_token().await.unwrap();
                }
                _ => error!("api vehicles err = {}", e),
            },
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
    }
}

pub async fn save_vehicle_data(d: &VehicleData) -> Result<(), std::io::Error> {
    let p = format!(".cache/{}/vehicle_data.json", d.vehicle_id);
    std::fs::write(&p, serde_json::to_string_pretty(d).unwrap())?;
    Ok(())
}
