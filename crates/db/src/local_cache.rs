use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use base::pb::tesla::*;
use prost::Message;
use std::fs::File;
use std::io::{Read, Write};
use base::*;

pub struct LocalStream {
    f: File,
}

impl LocalStream {
    pub fn new(vehicle_id: i64) -> Self {
        check_make_dir(&format!(".cache/{}", vehicle_id));
        let stream_path = format!(".cache/{}/stream.dat", vehicle_id,);
        let f = std::fs::File::options()
            .create(true)
            .append(true)
            .open(&stream_path)
            .expect("open stream failed.");
        Self { f }
    }

    pub fn write(&mut self, vd: &VehiclePeriodRecord) -> Result<(), std::io::Error> {
        let mut b = vec![];
        vd.encode(&mut b).expect("pb encode unwrap");
        self.f.write_u32::<LittleEndian>(b.len() as u32).unwrap();
        self.f.write_all(&b)
    }

    pub fn load(vehicle_id: i64) -> Result<Vec<VehiclePeriodRecord>, std::io::Error> {
        let path = format!(".cache/{}/stream.dat", vehicle_id);
        let mut file = File::open(&path)?;
        let mut v = vec![];
        let mut b = vec![];
        loop {
            let l = file.read_u32::<LittleEndian>();
            if let Err(_e) = l {
                break;
            }
            let l = l.unwrap() as usize;
            b.clear();
            b.resize(l, 0);
            file.read(&mut b).expect("读取vd block失败");
            let vd = VehiclePeriodRecord::decode(b.as_ref()).unwrap();
            v.push(vd);
        }
        Ok(v)
    }
}

