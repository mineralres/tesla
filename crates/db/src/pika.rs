use crate::*;
use base::pb::tesla::*;
use chrono::NaiveDateTime;
use log::info;
use prost::Message;
use redis::AsyncCommands;

pub struct PikaConnection {
    conn: redis::aio::Connection,
}

impl PikaConnection {
    pub async fn connect(address: &str) -> Result<Self, Error> {
        Ok(PikaConnection {
            conn: redis::Client::open(address)?.get_async_connection().await?,
        })
    }

    pub async fn save_vehicle_period_record(
        &mut self,
        vid: i64,
        pr: &VehiclePeriodRecord,
    ) -> Result<(), Error> {
        let table = format!(
            "pr-{vid}-{}",
            NaiveDateTime::from_timestamp_opt(pr.timestamp, 0)
                .ok_or(Error::FromTimestampErr)?
                .format("%Y%m%d")
                .to_string()
        );
        info!("save_vehicle_period_record table={table}");
        let mut b = vec![];
        pr.encode(&mut b).unwrap();
        Ok(self.conn.hset(table, pr.timestamp, b).await?)
    }

    pub async fn load_daily_vehicle_period_records(
        &mut self,
        vid: i64,
        day: i32,
    ) -> Result<Vec<VehiclePeriodRecord>, Error> {
        let table = format!("pr-{vid}-{day}");
        info!("load_daily_vehicle_period_records table={table}");
        let arr: Vec<Vec<u8>> = self.conn.hvals(table).await?;
        let mut v = vec![];
        for buf in &arr {
            let pr = VehiclePeriodRecord::decode(buf.as_ref())?;
            v.push(pr);
        }
        Ok(v)
    }
}
