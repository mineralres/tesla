use tesla_api::ApiClientConfig;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Config {
    pub client_config: ApiClientConfig,
    pub http_port: u16,
    pub https_port: u16,
}

impl Config {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let c = serde_json::from_reader(reader)?;
        Ok(c)
    }
}
