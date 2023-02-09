use derive_more::{Display, From};
use itertools::Itertools;
use log::{error, info};
use pb::tesla::Vehicle;
use pb::tesla::*;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;

#[derive(Debug, Display, From)]
pub enum ApiError {
    ReqwestError(reqwest::Error),
    EmptyTeslaAuthSid,
    Unauthorized,
    VehicleUnavailable,
    VehicleOffline,
    StreamWebSocketClosed,
    InvalidEmail,
    InvalidPassword,
    LocalChannelClosed,
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

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct ApiClientConfig {
    pub api_root: String,
    pub stream_path: String,
    pub auth_root: String,
}

impl ApiClientConfig {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let c = serde_json::from_reader(reader)?;
        Ok(c)
    }
}

#[derive(Clone)]
pub struct ApiClient {
    pub cookie: String,
    pub conf: ApiClientConfig,
    pub token: AccessTokenResponse,
}
impl ApiClient {
    pub async fn init(cookie: &str, conf: &ApiClientConfig) -> Self {
        let file = std::fs::File::open(".cache/token.json");
        let token = match file {
            Ok(file) => {
                let reader = std::io::BufReader::new(file);
                let token: AccessTokenResponse = serde_json::from_reader(reader).unwrap();
                token
            }
            Err(_e) => AccessTokenResponse::default(),
        };
        ApiClient {
            cookie: cookie.to_string(),
            token: token,
            conf: conf.clone(),
        }
    }

    /// get access token
    pub async fn get_token(
        &mut self,
        email: &str,
        password: &str,
    ) -> Result<AccessTokenResponse, ApiError> {
        if email.is_empty() {
            return Err(ApiError::InvalidEmail);
        }
        if password.is_empty() {
            return Err(ApiError::InvalidPassword);
        }
        let url = format!("{}/oauth2/v3/authorize", self.conf.auth_root);
        let code_verifier = Alphanumeric.sample_string(&mut rand::thread_rng(), 86);
        let mut hasher = Sha256::new();
        hasher.update(&code_verifier);
        let result = hasher.finalize();
        let result: String = format!("{:x}", result);
        let base64_string = base64_url::encode(&result);
        let code_challenge = base64_string;
        let state = Alphanumeric.sample_string(&mut rand::thread_rng(), 20);
        let client_id = "ownerapi";
        let scope = "openid email offline_access";
        let response_type = "code";
        let client = reqwest::Client::builder()
            .user_agent("tesla-api")
            .build()
            .unwrap();
        let resp = client
            .get(url)
            .query(&[
                ("client_id", client_id),
                ("code_challenge", &code_challenge),
                ("code_challenge_method", "S256"),
                ("redirect_uri", "https://auth.tesla.com/void/callback"),
                ("response_type", response_type),
                ("scope", scope),
                ("state", &state),
                ("login_hint", &email),
            ])
            .header("User-Agent", "tesla-api")
            .send()
            .await?;
        let mut c1 = "".to_string();
        for (header_name, header_value) in resp.headers() {
            if header_name == "set-cookie" {
                c1 = header_value.to_str().unwrap().to_string();
                break;
            }
        }
        if c1.is_empty() {
            return Err(ApiError::EmptyTeslaAuthSid);
        }
        let html = resp.text().await?;
        let form_start_index = html
            .find(r#"<form method="post" id="form" class="sso-form sign-in-form">"#)
            .unwrap();
        let html = &html[form_start_index..];
        let mut input_tags = html
            .lines()
            .filter(|l| l.contains("<input") && l.contains("hidden"))
            .map(|l| {
                let arr = l
                    .trim()
                    .split(" ")
                    .filter(|s| s.contains("="))
                    .map(|s| {
                        s.split("=")
                            .map(|s| s.trim_matches('\"'))
                            .collect::<Vec<_>>()
                    })
                    .filter(|v| v.len() == 2)
                    .collect::<Vec<_>>();
                arr
            })
            .filter(|arr| arr.len() == 3)
            .map(|arr| (arr[1][1], arr[2][1]))
            .take(4)
            .collect_vec();
        input_tags.push(("cancel", ""));
        let url = format!("{}/oauth2/v3/authorize", self.conf.auth_root);
        let mut params = HashMap::new();
        params.insert("identity", email);
        params.insert("credential", password);
        for it in &input_tags {
            params.insert(it.0, it.1);
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("tesla-api")
            .build()
            .unwrap();
        let builder = client
            .post(url)
            .header("cookie", &c1)
            .header("User-Agent", "tesla-api")
            .query(&[
                ("client_id", client_id),
                ("code_challenge", &code_challenge),
                ("code_challenge_method", "S256"),
                ("redirect_uri", "https://auth.tesla.com/void/callback"),
                ("response_type", response_type),
                ("scope", scope),
                ("state", &state),
            ])
            .form(&params);
        let resp = builder.send().await?;
        let mut redirect_location = "".to_string();
        for (hn, hv) in resp.headers() {
            if hn == "location" {
                redirect_location = hv.to_str().unwrap().to_string();
                break;
            }
        }
        let mut code = "".to_string();
        for p in reqwest::Url::parse(&redirect_location)
            .unwrap()
            .query_pairs()
        {
            if p.0 == "code" {
                code = p.1.to_string();
            }
        }
        info!("code={:?}", code);
        let url = format!("{}/oauth2/v3/token", self.conf.auth_root);
        #[derive(Debug, Serialize)]
        struct SReq {
            grant_type: String,
            client_id: String,
            code: String,
            code_verifier: String,
            redirect_uri: String,
        }
        let req = SReq {
            grant_type: "authorization_code".to_string(),
            client_id: client_id.to_string(),
            code: code,
            code_verifier: code_verifier,
            redirect_uri: "https://auth.tesla.com/void/callback".to_string(),
        };
        let mut resp = client
            .post(url)
            .json(&req)
            .send()
            .await?
            .json::<AccessTokenResponse>()
            .await?;
        resp.create_timestamp = Some(chrono::Local::now().timestamp());
        std::fs::write(
            ".cache/token.json",
            serde_json::to_string_pretty(&resp).unwrap(),
        )
        .unwrap();
        self.token = resp.clone();
        Ok(resp)
    }

    /// refresh token
    pub async fn refresh_token(&mut self) -> Result<(), ApiError> {
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
        info!("old token={:?}", self.token);
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
        info!("new token={:?}", resp);
        std::fs::write(
            ".cache/token.json",
            serde_json::to_string_pretty(&resp).unwrap(),
        )
        .unwrap();
        self.token = resp;
        Ok(())
    }

    pub async fn charge_state(&self) -> Result<ChargeResponse, ApiError> {
        // let url = "https://www.tesla.cn/teslaaccount/charging/api/history";
        let url = "https://www.tesla.cn/teslaaccount/charging/api/history?startTime=2022-01-21T03%3A29%3A45.613Z&endTime=2023-01-21T03%3A29%3A45.613Z";

        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header("cookie", &self.cookie)
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(ApiError::Unauthorized);
        }
        let resp_data = resp.json::<ChargeResponse>().await?;
        Ok(resp_data)
    }

    pub async fn users_me(&self) -> Result<UsersMeResponse, ApiError> {
        let url = self.conf.api_root.to_string() + "/api/1/users/me";

        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: UsersMeResponse,
        }
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.access_token),
            )
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(ApiError::Unauthorized);
        }
        let resp = resp.json::<XResponse>().await?;
        Ok(resp.response)
    }

    pub async fn vehicles(&self) -> Result<Vec<Vehicle>, ApiError> {
        let url = self.conf.api_root.to_string() + "/api/1/vehicles";

        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: Vec<Vehicle>,
            count: i32,
        }
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.access_token),
            )
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(ApiError::Unauthorized);
        }
        let resp = resp.json::<XResponse>().await?;
        if resp.response.len() as i32 != resp.count {
            panic!("resp={:?}", resp);
        }
        Ok(resp.response)
    }

    pub async fn vehicle_data(&self, id: i64) -> Result<VehicleData, ApiError> {
        let url = format!("{}/api/1/vehicles/{id}/vehicle_data", self.conf.api_root);
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: Option<VehicleData>,
            // error: Option<String>,
        }
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.access_token),
            )
            .send()
            .await?;
        if resp.status() == 401 {
            return Err(ApiError::Unauthorized);
        }
        let resp = resp.json::<XResponse>().await?;
        if let Some(resp) = resp.response {
            return Ok(resp);
        }
        return Err(ApiError::VehicleUnavailable);
    }

    pub async fn wake_up(&self, id: i64) -> Result<VehicleData, ApiError> {
        let url = format!("{}/api/1/vehicles/{id}/wake_up", self.conf.api_root);
        #[derive(Debug, Deserialize)]
        struct XResponse {
            response: VehicleData,
        }
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.token.access_token),
            )
            .send()
            .await?
            .json::<XResponse>()
            .await?;
        Ok(resp.response)
    }

    pub async fn stream(
        &self,
        vehicle_id: i64,
        output: &tokio::sync::mpsc::Sender<DrivingState>,
    ) -> Result<(), ApiError> {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message};
        #[derive(Debug, Serialize)]
        struct ConnectMessage {
            msg_type: String,
            token: String,
            value: String,
            tag: String,
        }
        let connect_message = ConnectMessage {
            msg_type:"data:subscribe_oauth".to_string(),
            token: self.token.access_token.clone(),
            value:"speed,odometer,soc,elevation,est_heading,est_lat,est_lng,power,shift_state,range,est_range,heading".to_string(),
            tag: format!("{vehicle_id}")
        };
        let json = serde_json::to_string(&connect_message).unwrap();
        let (mut ws_stream, _) = connect_async_tls_with_config(&self.conf.stream_path, None, None)
            .await
            .unwrap();
        ws_stream.send(Message::text(json.clone())).await.unwrap();
        // ws_stream.send(Message::Ping).await.unwrap();
        // tokio::spawn(async {
        //     ws_stream.send(Message::Ping(())).await.unwrap();
        //     tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
        // });
        #[derive(Debug, Deserialize, Serialize)]
        struct StreamMessage {
            msg_type: String,
            tag: Option<String>,
            value: Option<String>,
            error_type: Option<String>,
        }
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
                                update.timestamp = it / 1000;
                                update.milliseconds = it % 1000;
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
                                    return Err(ApiError::LocalChannelClosed);
                                }
                            }
                            "data:error" => {
                                if msg.error_type.is_some() {
                                    match msg.error_type.as_ref().unwrap().as_str() {
                                        "vehicle_disconnected" => {
                                            ws_stream
                                                .send(Message::text(json.clone()))
                                                .await
                                                .unwrap();
                                        }
                                        "vehicle_error" => {
                                            if let Some(value) = &msg.value {
                                                if value.contains("Vehicle is offline") {
                                                    return Err(ApiError::VehicleOffline);
                                                }
                                            }
                                        }
                                        "client_error" => {
                                            if let Some(value) = &msg.value {
                                                if value.contains("Can't validate token.")
                                                    || value.contains("unauthorized")
                                                {
                                                    return Err(ApiError::Unauthorized);
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
                    return Err(ApiError::StreamWebSocketClosed);
                }
            }
        }

        Ok(())
    }
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
    use super::*;

    #[tokio::test]
    async fn it_works() {}
}
