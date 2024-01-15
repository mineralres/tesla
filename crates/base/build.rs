fn main() {
	println!("cargo:rerun-if-changed=./protos");
	tonic_build::configure()
	    .type_attribute(".", "#[derive(serde_derive::Serialize, serde_derive::Deserialize)]")
	    .protoc_arg("--experimental_allow_proto3_optional")
	    .compile(&["./protos/base.proto", "./protos/tesla.proto"], &["./protos"])
	    .unwrap();
    }
    