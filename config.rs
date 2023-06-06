use serde::Deserialize;
// use crate::redis_interop::ffi::StatsConfig;

#[derive(Deserialize)]
pub struct MySqlConfig {
    pub host: String,
    pub db_name: String,
    pub user: String,
    pub pass: String,
}

#[derive(Deserialize)]
pub struct RedisConfig
{
    pub host: String,
    pub db_index: u8,
    pub hashrate_ttl_seconds: u32
}

#[derive(Deserialize)]
pub struct CoinConfig {
    pub redis: RedisConfig,
    pub mysql: MySqlConfig,
    pub stats: StatsConfig,
    pub min_payout_threshold: u64,
    pub pow_fee: f64
}

#[derive(Deserialize)]
pub struct StatsConfig {
    pub hashrate_interval_seconds: u32,
    pub effort_interval_seconds: u32,
    pub average_hashrate_interval_seconds: u32,
    pub mined_blocks_interval: u32,
}