#![deny(unsafe_code)]
mod currency;

use clap::{arg, command, ArgAction, Args, Parser, Subcommand};

use log::info;
use log4rs;

use sickpool2lib::coins::bitcoin::Btc;
use sickpool2lib::coins::coin::Coin;
use sickpool2lib::config::{ProtocolServerConfig, ServerConfig};
use sickpool2lib::p2p::networking::block::Block;
use sickpool2lib::p2p::networking::config::ConfigP2P;
use sickpool2lib::p2p::networking::hard_config::DEFAULT_STRATUM_CREATE_POOL_PORT;
use sickpool2lib::p2p::networking::protocol::ProtocolP2P;
use sickpool2lib::p2p::networking::server::ServerP2P;

use sickpool2lib::protocol::{JsonRpcProtocol, Protocol};
use sickpool2lib::stratum::header::BlockHeader;
use sickpool2lib::stratum::server::StratumServer;
use sickpool2lib::stratum::stratum_v1::StratumV1;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::result::Result;

use std::fs;
use std::sync::Arc;

extern crate sickpool2lib;

use sickpool2lib::stratum::config::StratumConfig;

// type StratumV1Json = JsonRpcProtocol<StratumV1<bitcoincore_rpc::Client>, StratumV1ErrorCodes>;
fn read_config<T: serde::de::DeserializeOwned + serde::Serialize>(
    path: &Box<Path>,
    default: impl FnOnce() -> T,
) -> Result<T, String> {
    let data = match fs::read_to_string(path) {
        Ok(k) => k,
        Err(e) => {
            let _ = fs::create_dir_all(path.parent().unwrap());
            fs::write(
                path,
                serde_json::to_string_pretty(&default()).unwrap().as_bytes(),
            )
            .expect(&format!(
                "Failed to write default config file at {}",
                path.display()
            ));

            return Err(format!("Missing config file at: {}, generated default config, modify it and restart to continue. {}", path.display(), e));
        }
    };
    match serde_json::from_str(&data) {
        Ok(k) => Ok(k),
        Err(e) => Err(format!("Invalid config at: {}, {}", path.display(), e)),
    }
}

#[derive(Args, Clone, Debug)]
struct CreatePoolParams {
    #[clap(long)]
    pub name: String,

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
    /// Sets a custom directory path for the data and configuration files
    #[arg(short, long, value_name = "datadir")]
    datadir: Option<PathBuf>,
    #[clap(long, short, action=ArgAction::SetFalse)]
    list_pools: bool,
    // create a pool under this one with the following params:
    #[command(subcommand)]
    command: Option<SubCommands>,
}

fn main() -> Result<(), String> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();

    let cli = Cli::parse();

    let buf = if let Some(buf) = cli.datadir {
        buf
    } else {
        PathBuf::from("./data")
    };

    let stratum_cfg_path = buf.join("config/stratum.json").into_boxed_path();

    let stratum_config: ProtocolServerConfig<StratumConfig> =
        read_config(&stratum_cfg_path, || Btc::default_stratum_config())?;

    if let Some(cmd) = cli.command {
        if let SubCommands::CreatePool(params) = cmd {
            create_pool(buf, params, stratum_config);
        }
        return Ok(());
    }

    let p2p_cfg_path = buf.join("config/p2p.json").into_boxed_path();

    let p2p_config: ProtocolServerConfig<ConfigP2P<bitcoin::Block>> =
        read_config(&p2p_cfg_path, || {
            <Btc as Coin>::main_pool_config(buf.into_boxed_path().clone())
        })?;

    info!("Stratum config: {:#?}", &stratum_config);
    info!("P2P config: {:#?}", &p2p_config);

    let mut p2p_server: ServerP2P<Btc> = ServerP2P::new(p2p_config);

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

fn create_pool(
    data_dir: PathBuf,
    params: CreatePoolParams,
    stratum_config: ProtocolServerConfig<StratumConfig>,
) {
    info!("Creating pool with: {:#?}", params);

    let rpc_url = stratum_config.protocol_config.rpc_url;
    // unfinished config, need to mine the first share.
    let mut new_config = ProtocolP2P::<Btc>::get_new_pool_config(
        data_dir.clone().into_boxed_path(),
        params.name.clone(),
        rpc_url.clone(),
        params.diff1,
        1000,
    );

    let mut genesis_block_find_config = new_config.clone();
    genesis_block_find_config.consensus.diff_adjust_blocks = u32::MAX;

    let p2p_protocol = Arc::new(ProtocolP2P::new(genesis_block_find_config));

    let stratum_config = ProtocolServerConfig {
        server_config: ServerConfig {
            address: SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                DEFAULT_STRATUM_CREATE_POOL_PORT,
            ),
            processing_threads: 1,
        },
        protocol_config: StratumConfig {
            rpc_url,
            // we only need a single job, one share.
            job_poll_interval_ms: 1000,
            default_diff_units: params.diff1,
        },
    };
    let pool_name = params.name;

    let mut stratum_server: StratumServer<JsonRpcProtocol<StratumV1>> =
        StratumServer::new(stratum_config, p2p_protocol.clone());

    loop {
        stratum_server.process_stratum();

        let tip = p2p_protocol.block_manager.p2p_tip();
        let tip = &tip.inner;
        
        let encoded_height = tip.encoded.height;
        if encoded_height > 0 {
            let genesis = tip.block.clone();
            info!(
                "Found genesis block for new pool: {}, {:#?}",
                genesis.get_header().get_hash(),
                genesis,
            );

            new_config.consensus.genesis_block = genesis;

            let pool_hash = new_config.consensus.pool_id();
            info!("Pool hash: {}", pool_hash);

            let mut pool_path = data_dir.clone();
            pool_path.push("pools");
            pool_path.push(pool_name);

            let _ = fs::create_dir_all(&pool_path);
            pool_path.push("p2p.json");
            let s = serde_json::to_string_pretty(&new_config).unwrap();
            fs::write(pool_path, s).unwrap();
            break;
        }
    }
}
