#![deny(unsafe_code)]
mod currency;

use bitcoincore_rpc::Client;
use clap::{arg, command, ArgAction, Args, Parser, Subcommand};
use crypto_bigint::U256;
use log::info;
use log4rs;
use serde::{Deserialize, Serialize};
use sickpool2lib::address::Address;
use sickpool2lib::coins::bitcoin::{Btc, MyBtcAddr};
use sickpool2lib::coins::coin::Coin;
use sickpool2lib::config::{ProtocolServerConfig, ServerConfig};
use sickpool2lib::p2p::networking::block::Block;
use sickpool2lib::p2p::networking::config::{ConfigP2P, ConsensusConfigP2P};
use sickpool2lib::p2p::networking::hard_config::{DEFAULT_STRATUM_PORT, DEV_ADDRESS_BTC_STR, DEFAULT_STRATUM_CREATE_POOL_PORT};
use sickpool2lib::p2p::networking::protocol::ProtocolP2P;
use sickpool2lib::p2p::networking::server::ServerP2P;

use sickpool2lib::protocol::{JsonRpcProtocol, Protocol};
use sickpool2lib::stratum::header::BlockHeader;
use sickpool2lib::stratum::job_fetcher::BlockFetcher;
use sickpool2lib::stratum::server::StratumServer;
use sickpool2lib::stratum::stratum_v1::StratumV1;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::result::Result;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

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

#[derive(Args, Clone, Debug)]
struct CreatePoolParams {
    #[clap(long)]
    pub diff1: u64,
    #[clap(long)]
    pub block_time_ms: u64,
    #[clap(long)]
    pub diff_adjust_blocks: u32,
}

#[derive(Subcommand)]
enum SubCommands {
    CreatePool(CreatePoolParams),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "datadir", default_value=default_datadir().into_os_string())]
    datadir: PathBuf,
    #[clap(long, short, action=ArgAction::SetFalse)]
    list_pools: bool,
    // create a pool under this one with the following params:
    #[command(subcommand)]
    create_pool: Option<SubCommands>,
}

fn main() -> Result<(), String> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let cli = Cli::parse();

    let buf = cli.datadir;

    let stratum_cfg_path =
        PathBuf::from_iter([&buf, &"config/stratum.json".parse().unwrap()].iter())
            .into_boxed_path();

    let stratum_config: ProtocolServerConfig<StratumConfig> = read_config(
        &stratum_cfg_path,
        &ProtocolServerConfig::<StratumConfig>::default(),
    )?;

    if let Some(create_pool) = cli.create_pool {
        if let SubCommands::CreatePool(create_pool) = create_pool {
            info!("Creating pool with: {:#?}", create_pool);

            let rpc_url = stratum_config.protocol_config.rpc_url;
            // unfinished config, need to mine the first share.
            let new_config = ProtocolP2P::<Btc>::get_new_pool_config(rpc_url.clone(), 0, 1);
            let p2p_protocol = Arc::new(ProtocolP2P::new((new_config, buf.into_boxed_path(), 1)));

            let stratum_config = ProtocolServerConfig {
                server_config: ServerConfig {
                    address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_STRATUM_CREATE_POOL_PORT),
                    processing_threads: 1,
                },
                protocol_config: StratumConfig {
                    rpc_url,
                    // we only need a single job, one share.
                    job_poll_interval: Duration::MAX,
                    default_diff_units: create_pool.diff1,
                },
            };

            let mut stratum_server: StratumServer<JsonRpcProtocol<StratumV1>> =
                StratumServer::new(stratum_config, p2p_protocol.clone());

            loop {
                stratum_server.process_stratum();

                let tip = p2p_protocol.block_manager.p2p_tip();
                if tip.encoded.prev_hash != U256::ZERO {
                    info!("Found genesis block for new pool: {}", tip.block.get_header().get_hash());

                    break;
                }
            }
        }
        return Ok(());
    }

    let p2p_cfg_path =
        PathBuf::from_iter([&buf, &"config/p2p.json".parse().unwrap()].iter()).into_boxed_path();

    let p2p_config: ProtocolServerConfig<ConfigP2P<bitcoin::Block>> =
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
