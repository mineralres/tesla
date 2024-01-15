pub mod local_cache;
pub mod pika;

#[derive(Debug, derive_more::Display, derive_more::From)]
pub enum Error {
    IoErr(std::io::Error),
    RedisErr(redis::RedisError),
    InvalidVehicleState,
    FromTimestampErr,
    DecodeErr(prost::DecodeError),
    EncodeErr(prost::EncodeError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
