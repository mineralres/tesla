use base::pb::tesla::*;
use futures_util::{SinkExt, StreamExt};
use itertools::Itertools;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message};

#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    ReqwestError(reqwest::Error),
    EmptyTeslaAuthSid,
    Unauthorized,
    VehicleUnavailable,
    VehicleOffline,
    StreamWebSocketClosed,
    InvalidEmail,
    InvalidPassword,
    LocalChannelClosed,
    AuthFailed(String),
    WsErr(tokio_tungstenite::tungstenite::Error),
    AccessTokenExpired,
    SerdeJsonErr(serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub state: Option<String>,
    pub token_type: String,
    pub create_timestamp: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UsersMeResponse {
    pub email: String,
    pub full_name: String,
    pub profile_image_url: String,
}

pub struct TokenState {
    pub cookie: String,
    token: AccessTokenResponse,
    conf: ApiConfig,
}

impl TokenState {
    pub async fn new(conf: &ApiConfig, cookie: String) -> Result<Self, Error> {
        let file = std::fs::File::open(".cache/token.json");
        let token = match file {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                let token: AccessTokenResponse = serde_json::from_reader(reader).unwrap();
                token
            }
            Err(_e) => AccessTokenResponse::default(),
        };
        Ok(Self {
            token,
            conf: conf.clone(),
            cookie,
        })
    }

    fn cache_token(token: &AccessTokenResponse) -> std::io::Result<()> {
        std::fs::write(
            ".cache/token.json",
            serde_json::to_string_pretty(&token).unwrap(),
        )
    }

    /// refresh token
    pub async fn refresh_token(&mut self) -> Result<(), Error> {
        info!("start refresh_token ");
        let url = format!("{}/oauth2/v3/token", self.conf.auth_root);
        #[derive(Debug, Serialize)]
        struct SReq {
            grant_type: String,
            client_id: String,
            refresh_token: String,
            scope: String,
        }
        let client_id = "ownerapi";
        let scope = "openid email offline_access";
        let req = SReq {
            grant_type: "refresh_token".to_string(),
            client_id: client_id.to_string(),
            refresh_token: self.token.refresh_token.clone(),
            scope: scope.to_string(),
        };
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("tesla-api")
            .build()
            .unwrap();
        let mut resp = client
            .post(url)
            .json(&req)
            .send()
            .await?
            .json::<AccessTokenResponse>()
            .await?;
        resp.create_timestamp = Some(chrono::Local::now().timestamp());
        TokenState::cache_token(&resp).expect("");
        info!("new token ={:?}", resp);
        self.token = resp;
        Ok(())
    }

    pub fn set_api_token(
        &mut self,
        access_token: &str,
        refresh_token: &str,
    ) -> std::io::Result<()> {
        self.token.access_token = access_token.to_string();
        self.token.refresh_token = refresh_token.to_string();
        self.token.create_timestamp = Some(chrono::Local::now().timestamp());
        TokenState::cache_token(&self.token)
    }

    pub fn is_valid(&self) -> bool {
        self.token.access_token.len() > 0 && self.token.refresh_token.len() > 0
    }

    pub async fn check_refresh_token(&mut self) -> Result<(), Error> {
        let nowts = chrono::Local::now().timestamp();
        if let Some(ct) = self.token.create_timestamp {
            info!("token will be expired in {}", ct + self.token.expires_in - nowts);
            if ct + self.token.expires_in - nowts < 1800 {
                info!("now need to refresh token");
            } else {
                return Ok(());
            }
        }
        info!("check_refresh_token 2");
        self.refresh_token().await
    }
}

#[derive(Clone)]
pub struct ApiClient {
    pub conf: ApiConfig,
    pub token: std::sync::Arc<tokio::sync::Mutex<TokenState>>,
}
impl ApiClient {
    pub async fn init(
        conf: &ApiConfig,
        token: std::sync::Arc<tokio::sync::Mutex<TokenState>>,
    ) -> Self {
        ApiClient {
            conf: conf.clone(),
            token,
        }
    }

    async fn make_api_request_builder(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.conf.api_root.to_string(), path);
        let access_token = { self.token.lock().await.token.access_token.clone() };
        reqwest::Client::new()
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
    }

    pub async fn charge_state(&self) -> Result<ChargeResponse, Error> {
        // let url = "https://www.tesla.cn/teslaaccount/charging/api/history";
        let url = "https://www.tesla.cn/teslaaccount/charging/api/history?startTime=2022-01-21T03%3A29%3A45.613Z&endTime=2023-01-21T03%3A29%3A45.613Z";

        let client = reqwest::Client::new();
        let resp = client.get(url).send().await?;
        if resp.status() == 401 {
            return Err(Error::Unauthorized);
        }
        let resp_data = resp.json::<ChargeResponse>().await?;
        Ok(resp_data)
    }

    pub async fn users_me(&self) -> Result<UsersMeResponse, Error> {
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: UsersMeResponse,
        }
        let resp = self
            .make_api_request_builder("/api/1/users/me")
            .await
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(Error::Unauthorized);
        }
        let resp = resp.json::<XResponse>().await?;
        Ok(resp.response)
    }

    pub async fn vehicles(&self) -> Result<Vec<Vehicle>, Error> {
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: Vec<Vehicle>,
            count: i32,
        }
        let resp = self
            .make_api_request_builder("/api/1/vehicles")
            .await
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(Error::Unauthorized);
        }
        // let text = resp.text().await?;
        // let j: serde_json::Value = serde_json::from_str(&text).unwrap();
        // panic!("{}", serde_json::to_string_pretty(&j).unwrap());
        let resp = resp.json::<XResponse>().await?;
        if resp.response.len() as i32 != resp.count {
            panic!("resp={:?}", resp);
        }
        Ok(resp.response)
    }

    pub async fn vehicle_data(&self, id: i64) -> Result<VehicleData, Error> {
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: Option<VehicleData>,
            //            error: Option<String>,
            //           error_description: Option<String>,
        }
        let resp = self
            .make_api_request_builder(&format!("/api/1/vehicles/{id}/vehicle_data"))
            .await
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(Error::Unauthorized);
        }
        let text = resp.text().await?;
        // let j: serde_json::Value = serde_json::from_str(&text).unwrap();
        // panic!("{}", serde_json::to_string_pretty(&j).unwrap());
        let resp = serde_json::from_str::<XResponse>(&text);
        if let Err(e) = &resp {
            error!("{text}");
            error!("{e}");
        }
        let resp = resp?;
        // let resp = resp.json::<XResponse>().await?;
        if let Some(resp) = resp.response {
            return Ok(resp);
        }
        return Err(Error::VehicleUnavailable);
    }

    pub async fn wake_up(&self, id: i64) -> Result<VehicleData, Error> {
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: VehicleData,
        }
        let resp = self
            .make_api_request_builder(&format!("/api/1/vehicles/{id}/wake_up"))
            .await
            .send()
            .await?
            .json::<XResponse>()
            .await?;
        Ok(resp.response)
    }

    pub async fn make_ws_connect_message(
        vehicle_id: i64,
        token: &Arc<Mutex<TokenState>>,
    ) -> ConnectMessage {
        let access_token = {
            let t = token.lock().await;
            t.token.access_token.clone()
        };
        ConnectMessage {
            msg_type:"data:subscribe_oauth".to_string(),
            token: access_token,
            value:"speed,odometer,soc,elevation,est_heading,est_lat,est_lng,power,shift_state,range,est_range,heading".to_string(),
            tag: format!("{vehicle_id}")
        }
    }

    pub async fn prepare_stream(
        vehicle_id: i64,
        token: &Arc<Mutex<TokenState>>,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Error,
    > {
        let (stream_path, is_token_valid) = {
            let t = token.lock().await;
            (t.conf.stream_path.clone(), t.is_valid())
        };
        if !is_token_valid {
            return Err(Error::AccessTokenExpired);
        }
        let connect_message = ApiClient::make_ws_connect_message(vehicle_id, token).await;
        let json = serde_json::to_string(&connect_message).unwrap();
        let (mut ws_stream, _) =
            connect_async_tls_with_config(&stream_path, None, true, None).await?;
        ws_stream.send(Message::text(json.clone())).await.unwrap();
        Ok(ws_stream)
    }

    pub async fn stream(
        &self,
        vehicle_id: i64,
        output: &tokio::sync::mpsc::Sender<DrivingState>,
    ) -> Result<(), Error> {
        let mut ws_stream = ApiClient::prepare_stream(vehicle_id, &self.token).await?;
        let log_path = format!(
            ".cache/{}/logs/{}.log",
            vehicle_id,
            chrono::Local::now().format("%Y_%m_%d"),
        );
        let mut f = std::fs::File::options()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("open append.log failed.");
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
                                let json = serde_json::to_string(&update).unwrap();
                                f.write(json.as_bytes()).unwrap();
                                f.write(b"\r\n").unwrap();
                                if let Err(_e) = output.send(update).await {
                                    return Err(Error::LocalChannelClosed);
                                }
                            }
                            "data:error" => {
                                if msg.error_type.is_some() {
                                    match msg.error_type.as_ref().unwrap().as_str() {
                                        "vehicle_disconnected" => {
                                            // let connect_message =
                                            //     self.make_ws_connect_message(vehicle_id).await;
                                            // let json =
                                            //     serde_json::to_string(&connect_message).unwrap();
                                            // ws_stream
                                            //     .send(Message::text(json.clone()))
                                            //     .await
                                            //     .unwrap();
                                        }
                                        "vehicle_error" => {
                                            if let Some(value) = &msg.value {
                                                if value.contains("Vehicle is offline") {
                                                    return Err(Error::VehicleOffline);
                                                }
                                            }
                                        }
                                        "client_error" => {
                                            if let Some(value) = &msg.value {
                                                if value.contains("Can't validate token.")
                                                    || value.contains("unauthorized")
                                                {
                                                    return Err(Error::Unauthorized);
                                                }
                                            }
                                        }
                                        _ => error!("error_msg={:?}", msg),
                                    }
                                    let json = serde_json::to_string(&msg).unwrap();
                                    f.write(json.as_bytes()).unwrap();
                                    f.write(b"\r\n").unwrap();
                                }
                            }
                            "control:hello" => {}
                            _ => {
                                info!("unkown msg type, msg={:?}", msg);
                            }
                        }
                    } else {
                        info!("receive msg={:?}", msg.into_text());
                    }
                }
                Err(_e) => {
                    return Err(Error::StreamWebSocketClosed);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ConnectMessage {
    pub msg_type: String,
    pub token: String,
    pub value: String,
    pub tag: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StreamMessage {
    pub msg_type: String,
    pub tag: Option<String>,
    pub value: Option<String>,
    pub error_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingPackage {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingCredit {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargingFeeSession {
    pub session_fee_id: i64,
    pub currency_code: String,
    pub fee_type: String,
    pub is_paid: bool,
    pub net_due: f64,
    pub pricing_type: String,
    pub process_flag_id: i64,
    pub rate_base: f64,
    pub rate_tier1: f64,
    pub rate_tier2: f64,
    pub status: String,
    pub total_base: f64,
    pub total_due: f64,
    pub uom: String,
    pub usage_base: f64,
    pub usage_tier1: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeRecord {
    pub session_id: i64,
    pub billing_type: String,
    pub cabinet_id: String,
    pub charge_session_id: String,
    pub charge_start_date_time: String,
    pub charge_stop_date_time: String,
    pub charging_package: Option<ChargingPackage>,
    pub charging_site_type: String,
    pub credit: Option<ChargingCredit>,
    pub fees: Vec<ChargingFeeSession>,
    pub din: String,
    pub is_dc_enforced: bool,
    pub post_id: String,
    pub program_type: String,
    pub site_location_name: String,
    pub unlatch_date_time: String,
    pub vehicle_make_type: String,
    pub vin: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeResponse {
    pub code: i32,
    pub message: String,
    pub success: bool,
    pub data: Vec<ChargeRecord>,
}

#[cfg(test)]
mod tests {
    use base::init_logger;

    use super::*;

    #[tokio::test]
    async fn it_works() {
        init_logger();
        let url = format!("https://owner-api.vn.cloud.tesla.cn/api/1/vehicles");
        // let url = "https://auth.tesla.com/oauth2/v3/authorize?client_id=ownerapi&code_challenge=yu9aUhsjkBBC7-ccqYclglsEGadmFArQ4R8jOykUYVA&code_challenge_method=S256&redirect_uri=https%3A%2F%2Fauth.tesla.com%2Fvoid%2Fcallback&response_type=code&scope=openid+email+offline_access&state=ODJiNzQ4ZGZkMTJm";
        let resp = reqwest::Client::new().get(&url).send().await.unwrap();
        let hs = resp.headers().clone();
        for (n, v) in &hs {
            info!("{n}: {:?}", v);
        }
        let status_code = resp.status();
        let text = resp.text().await.unwrap();
        info!("{text} status={status_code}");
    }
}
