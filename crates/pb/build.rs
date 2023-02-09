use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=./protos/");
    let base_dir = "./protos";
    let src_dirs = vec!["tesla"];
    let mut protos = vec![];
    for dir in src_dirs {
        let dx = Path::new(base_dir).join(&dir);
        for entry in fs::read_dir(&dx).expect("not this dir") {
            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                protos.push(path);
            }
        }
    }
    let includes = vec![PathBuf::from(base_dir)];
    tonic_build::configure()
        .type_attribute(
            ".",
            "#[derive(serde_derive::Serialize, serde_derive::Deserialize)]",
        )
        .compile(&protos, &includes)
        .unwrap();
}
