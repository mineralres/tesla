use crate::Error;
use base::pb::{base::*, tesla::*};
use db::pika::*;
use futures_util::StreamExt;
use itertools::Itertools;
use log::{error, info};
use std::sync::Arc;
use tesla_api::{ApiClient, StreamMessage};

pub struct VehicleMonitor {
    pub exit_sender: tokio::sync::oneshot::Sender<String>,
}

use async_stream::stream;
use futures_util::pin_mut;

impl VehicleMonitor {
    pub async fn init(api: ApiClient, vehicle: Vehicle, conf: AppConfig) -> Result<Self, Error> {
        info!("monitor startup ={:?}", vehicle);
        let (exit_sender, mut exit_receiver) = tokio::sync::oneshot::channel::<String>();
        let vm = Self { exit_sender };
        // 60秒检查一下DrivingState
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
        let vehicle_id = vehicle.vehicle_id;

        tokio::spawn(async move {
            use tesla_api::Error::*;
            let token = Arc::clone(&api.token);

            let mut pr = VehiclePeriodRecord::default();
            let s = stream! {
            loop {
                let ws_stream = ApiClient::prepare_stream(vehicle_id, &token).await;
                match ws_stream {
                    Ok(mut ws_stream) => {
                        info!("stream prepared.");
                        while let Some(msg) = ws_stream.next().await {
                            match msg {
                                    Ok(msg) => {
                                        if msg.is_text() || msg.is_binary() {
                                            let d = msg.into_data();
                                            let msg = serde_json::from_slice::<StreamMessage>(&d).expect("");
                                            match msg.msg_type.as_str() {
                                                "data:update" => {
                                                    let arr = msg.value.as_ref().unwrap().split(",").collect_vec();
                                                    let mut update = DrivingState::default();
                                                    let it = arr[0].parse::<i64>().expect("");
                                                    update.timestamp = it;
                                                    let pf = |s: &str| {
                                                        if s.len() == 0 {
                                                            0.0
                                                        } else {
                                                            s.parse::<f64>().expect("")
                                                        }
                                                    };
                                                    update.speed = if arr[1] == "" {
                                                        0.0
                                                    } else {
                                                        arr[1].parse::<f64>().expect("")
                                                    };
                                                    update.odometer = pf(arr[2]);
                                                    update.soc = pf(arr[3]);
                                                    update.elevation = pf(arr[4]);
                                                    update.est_heading = pf(arr[5]);
                                                    update.est_lat = pf(arr[6]);
                                                    update.est_lng = pf(arr[7]);
                                                    update.power = pf(arr[8]);
                                                    update.shift_state = arr[9].to_string();
                                                    update.range = pf(arr[10]);
                                                    update.est_range = pf(arr[11]);
                                                    update.heading = pf(arr[12]);
                                                    yield update;
                                                }
                                                "data:error" => {
                                                    if msg.error_type.is_some() {
                                                        match msg.error_type.as_ref().unwrap().as_str() {
                                                            "vehicle_disconnected" => {
                                                            }
                                                            "vehicle_error" => {
                                                                if let Some(value) = &msg.value {
                                                                    if value.contains("Vehicle is offline") {
                                                                        error!("Steram vehicle is offline");
                                                                    }
                                                                }
                                                            }
                                                            "client_error" => {
                                                                if let Some(value) = &msg.value {
                                                                    if value.contains("Can't validate token.")
                                                                        || value.contains("unauthorized")
                                                                    {
                                                                        error!("Stream unauthorized ");
                                                                    }
                                                                }
                                                            }
                                                            _ => error!("error_msg={:?}", msg),
                                                        }
                                                    }
                                                }
                                                "control:hello" => {}
                                                _ => {
                                                    info!("unkown msg type, msg={:?}", msg);
                                                }
                                            }
                                        } else {
                                            info!("ws update msg={:?}", msg.into_text());
                                        }
                                    }
                                    Err(e) => {
                                        error!("Stream closed: {e}");
                                        break;
                                    }
                                }
                        }
                        info!("ws stream closed");
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                    Err(e)=> {
                        error!("prepare_stream: {e}");
                        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                    }
                }
            }
            };
            pin_mut!(s);
            loop {
                tokio::select! {
                    _ = &mut exit_receiver => {
                        info!("Vehicle monitor exit loop");
                        break;
                    }
                    update = s.next() => {
                        if let Some(update) = update {
                            pr.updates.push(update);
                        }
                    }
                    _instant = ticker.tick() => {
                        let vehicle_data = api.vehicle_data(vehicle.id).await;
                        match vehicle_data {
                            Ok(d) => {
                                info!("vehicle state=[{}]", d.state);
                                cache_vehicle_data(&d)
                                    .await
                                    .expect("save vehicle data failed");
                                pr.timestamp = chrono::Local::now().timestamp();
                                pr.snapshot = Some(d);
                            }
                            Err(e) => {
                                match e {
                                    Unauthorized => {
                                        error!("Stream unauthorized, get new access token");
                                    }
                                    VehicleOffline => {
                                        // pr.state = "offline".to_string();
                                    }
                                    _ => {
                                    //    pr.state = "unavailable".to_string();
                                    }
                                }
                                error!("vehicle_data err=[{}]", e);
                            }
                        }
                        if pr.timestamp > 0 {
                            // 每次都重新连接以,否则会timeout error.
                            match PikaConnection::connect(&conf.pika_address).await {
                                Ok(mut pika) => {
                                    match pika.save_vehicle_period_record(vehicle_id,&pr).await {
                                        Ok(())=>(),
                                        Err(e)=> error!("pika.save_vehicle_period_record: {e}")
                                    }
                                    info!("Save pr updates count = {}", pr.updates.len());
                                    pr.timestamp = 0;
                                    pr.updates.clear();
                                    pr.snapshot = None;
                                }
                                Err(e) => error!("PikaConnection::connect: {e}")
                            }
                        }
                    }
                }
            }
        });

        Ok(vm)
    }
}

pub async fn cache_vehicle_data(d: &VehicleData) -> Result<(), std::io::Error> {
    let p = format!(".cache/{}/vehicle_data.json", d.vehicle_id);
    std::fs::write(&p, serde_json::to_string_pretty(d).unwrap())?;
    Ok(())
}
