
pub trait Currency {
    const ATOMIC_UNITS : u64;
}

struct BtcCurrency;

impl Currency for BtcCurrency {
    const ATOMIC_UNITS : u64 = bitcoincore_rpc::bitcoin::constants::COIN_VALUE;
}