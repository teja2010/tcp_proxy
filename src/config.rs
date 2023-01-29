use config::Config;
use config::File;
use serde::Deserialize;
use std::net::SocketAddr;
use std::process;
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
        Err(e) => {
            eprint!("error building config {e}");
            process::exit(-1);
        }
        Ok(c) => c,
    };
    match builder.try_deserialize::<TpConfig>() {
        Err(e) => {
            eprint!("error deserializing config {e}");
            process::exit(-1);
        }
        Ok(t) => t,
    }
}
