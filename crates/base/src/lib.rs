pub mod pb {
    pub mod tesla {
        tonic::include_proto!("tesla");
    }
    pub mod base {
        tonic::include_proto!("base");
        impl AppConfig {
            pub fn load(path: &str) -> Result<Self, crate::Error> {
                let file = std::fs::File::open(path)?;
                let reader = std::io::BufReader::new(file);
                let c = serde_json::from_reader(reader)?;
                Ok(c)
            }
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    IoErr(std::io::Error),
    SerdeJsonErr(serde_json::Error),
}

// 一般性初始化日志
pub fn init_logger() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::init();
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
