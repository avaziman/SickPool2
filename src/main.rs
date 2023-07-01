#![deny(unsafe_code)]
mod currency;

use clap::{arg, command, Parser};
use log::info;
use log4rs;
use sickpool2lib::coins::bitcoin::Btc;
use sickpool2lib::coins::coin::Coin;
use sickpool2lib::config::ProtocolServerConfig;
use sickpool2lib::p2p::networking::config::ConfigP2P;
use sickpool2lib::p2p::networking::server::ServerP2P;

use sickpool2lib::protocol::JsonRpcProtocol;
use sickpool2lib::stratum::server::StratumServer;
use sickpool2lib::stratum::stratum_v1::StratumV1;

use std::path::{Path, PathBuf};
use std::result::Result;

use std::{ fs};

extern crate sickpool2lib;

use sickpool2lib::stratum::config::StratumConfig;

// type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;
fn read_config<T: serde::de::DeserializeOwned + serde::Serialize>(
    path: &Box<Path>,
    default: &T,
) -> Result<T, String> {
    let data = match fs::read_to_string(path) {
        Ok(k) => k,
        Err(e) => {
            fs::write(
                path,
                serde_json::to_string_pretty(default).unwrap().as_bytes(),
            )
            .expect("Failed to write config default file");
            return Err(format!("Missing config file at: {}, generated default config, modify it and restart to continue. {}", path.display(), e));
        }
    };
    match serde_json::from_str(&data) {
        Ok(k) => Ok(k),
        Err(e) => Err(format!("Invalid config at: {}, {}", path.display(), e)),
    }
}

fn default_datadir() -> PathBuf {
    PathBuf::from("datadir")
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "datadir", default_value=default_datadir().into_os_string())]
    datadir: PathBuf,

}

fn main() -> Result<(), String> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let cli = Cli::parse();

    let buf = cli.datadir;

    let stratum_cfg_path =
        PathBuf::from_iter([&buf, &"config/stratum.json".parse().unwrap()].iter())
            .into_boxed_path();

    let p2p_cfg_path =
        PathBuf::from_iter([&buf, &"config/p2p.json".parse().unwrap()].iter()).into_boxed_path();

    let stratum_config: ProtocolServerConfig<StratumConfig> = read_config(
        &stratum_cfg_path,
        &ProtocolServerConfig::<StratumConfig>::default(),
    )?;

    let p2p_config: ProtocolServerConfig<ConfigP2P> =
        read_config(&p2p_cfg_path, &<Btc as Coin>::main_config_p2p())?;

    info!("Stratum config: {:#?}", &stratum_config);
    info!("P2P config: {:#?}", &p2p_config);

    let mut p2p_server: ServerP2P<Btc> = ServerP2P::new(p2p_config, buf.into_boxed_path());

    let mut stratum_server: StratumServer<JsonRpcProtocol<StratumV1>> =
        StratumServer::new(stratum_config, p2p_server.protocol.clone());

    let stratum_thread = std::thread::spawn(move || loop {
        stratum_server.process_stratum();
    });

    let p2p_thread = std::thread::spawn(move || loop {
        p2p_server.process_p2p();
    });

    stratum_thread.join().unwrap();
    p2p_thread.join().unwrap();

    Ok(())
}
