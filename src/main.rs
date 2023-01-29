mod config;
mod pipe;
use log::{debug, info};
use std::env;
use std::process;
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() {
    // Set RUST_LOG=trace to start debug logs
    pretty_env_logger::init();
    let config_file = parse_args();
    let config = config::read_config(config_file);
    debug!("config {:#?}", config);

    start_threads(config.pairs);
    info!("Started tcp proxy");

    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigterm.recv() => {},
        _ = signal::ctrl_c() => {},
    };
    info!("Stopping tcp proxy");
    process::exit(0);
}

fn start_threads(mut pairs: Vec<config::Pair>) {
    while let Some(pair) = pairs.pop() {
        tokio::spawn(pipe::open_pipe(pair));
    }
}

fn parse_args() -> String {
    let mut args: Vec<String> = env::args().collect();
    args.remove(0);
    debug!("read args {args:?}");
    if args.len() != 2 {
        eprint!("usage --config CONFIG_FILE, {args:?}");
        process::exit(-1);
    }

    let mut config_file: String = "".to_string();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--config" | "-c" => {
                config_file = args[idx + 1].clone();
                debug!("config file {}", config_file);
                idx += 2;
            }
            _ => {
                eprint!("unknown arg {:?}", args[idx]);
                process::exit(-1);
            }
        }
    }

    config_file
}
