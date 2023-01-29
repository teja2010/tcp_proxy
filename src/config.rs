use config::Config;
use config::File;
use serde::Deserialize;
use std::net::SocketAddr;
use std::vec::Vec;

#[derive(Deserialize, Debug)]
pub struct Pair {
    pub in_sock: SocketAddr,
    pub out_sock: SocketAddr,
}

#[derive(Deserialize, Debug)]
pub struct TpConfig {
    pub pairs: Vec<Pair>,
}

pub fn read_config(name: String) -> TpConfig {
    let builder = match Config::builder().add_source(File::with_name(&name)).build() {
        Err(e) => panic!("error building config {e}"),
        Ok(c) => c,
    };
    match builder.try_deserialize::<TpConfig>() {
        Err(e) => panic!("error deserializing config {e}"),
        Ok(t) => t,
    }
}
