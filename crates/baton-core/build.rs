fn main() {
    let proto = "../../proto/baton/v1/baton.proto";
    println!("cargo:rerun-if-changed={proto}");
    let fds = protox::compile([proto], ["../../proto"]).expect("compile protos");
    prost_build::compile_fds(fds).expect("generate prost types");
}
