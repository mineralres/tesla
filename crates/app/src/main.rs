use clap::Parser;
use log::{error, info};
use tesla_api::{ApiClient, TokenState};
mod http;
use base::pb::base::*;
use base::*;
use http::*;
mod vehicle_monitor;
use std::collections::HashMap;
use vehicle_monitor::*;

#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    IoErr(std::io::Error),
    DbErr(db::Error),
}

#[derive(Parser)]
#[clap()]
struct Opts {
    #[clap(short, long)]
    config: String,
}

#[tokio::main]
async fn main() {
    init_logger();
    let opts: Opts = Opts::parse();
    check_make_dir(".cache");
    let cookie = r#"gdp_user_id=gioenc-c5d09234,8ccd,5bd9,a37d,5e54ceaed440;"#;
    let conf = AppConfig::load(&opts.config).expect("");
    info!("start conf={:?}", conf);
    let token = TokenState::new(conf.api_config.as_ref().expect(""), cookie.into())
        .await
        .unwrap();
    let token = std::sync::Arc::new(tokio::sync::Mutex::new(token));
    {
        // HTTP 服务
        let conf = conf.clone();
        let client = ApiClient::init(
            conf.api_config.as_ref().expect(""),
            std::sync::Arc::clone(&token),
        )
        .await;
        tokio::spawn(async move {
            httpd(client, conf).await;
        });
    }
    let mut monitors: HashMap<i64, VehicleMonitor> = HashMap::new();
    {
        // 检测vehicles()并启动监控 & check refresh access token
        let conf = conf.clone();
        let client = ApiClient::init(
            conf.api_config.as_ref().expect(""),
            std::sync::Arc::clone(&token),
        )
        .await;
        loop {
            {
                match token.lock().await.check_refresh_token().await {
                    Ok(()) => (),
                    Err(e) => {
                        error!("Maybe it's someting wrong with your token, {e}");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        continue;
                    }
                }
            }
            let vehicles = client.vehicles().await;
            match vehicles {
                Ok(vehicles) => {
                    for v in vehicles.iter() {
                        if monitors.contains_key(&v.id) {
                            continue;
                        }
                        let token = std::sync::Arc::clone(&token);
                        let api = ApiClient::init(conf.api_config.as_ref().expect(""), token).await;
                        let vm = VehicleMonitor::init(api, v.clone(), conf.clone()).await;
                        match vm {
                            Ok(vm) => {
                                monitors.insert(v.id, vm);
                            }
                            Err(e) => {
                                error!("VehicleMonitor::init {}", e);
                            }
                        }
                    }
                    let delete_list = monitors
                        .iter()
                        .filter(|(k, _v)| vehicles.iter().find(|v| **k == v.id).is_none())
                        .map(|(k, _v)| k.clone())
                        .collect::<Vec<_>>();
                    for k in delete_list.iter() {
                        if let Some(vm) = monitors.remove(k) {
                            vm.exit_sender.send("exit".into()).unwrap();
                        }
                    }
                }
                Err(e) => match e {
                    tesla_api::Error::Unauthorized => {
                        info!("api.vehicles err {e}");
                    }
                    _ => error!("api.vehicles: {}", e),
                },
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }
}
