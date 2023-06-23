use crypto_bigint::{CheckedMul, NonZero, U256};


use super::{hard_config::{PPLNS_SHARE_UNITS_256}, pplns::Score};

// pub static DIFF1: U256 = U256::ZERO;
static DIFF1: U256 =
    U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

pub fn get_diff(hash: &U256) -> Score {
    let hash = NonZero::new(*hash).unwrap();
    let (quotient, _remainder) = DIFF1
        .checked_mul(&PPLNS_SHARE_UNITS_256)
        .unwrap()
        .div_rem(&hash);

    quotient.as_words()[0]
}

#[cfg(test)]
mod tests {
    use crypto_bigint::U256;

    use crate::p2p::networking::difficulty::get_diff;

    static DIFF1: U256 =
        U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

    // https://en.bitcoin.it/wiki/Difficulty
    #[test]
    fn bitcoin_diff() {
        let check =
            U256::from_be_hex("00000000000404CB000000000000000000000000000000000000000000000000");

        let result = get_diff(&check);
        assert_eq!(result, 16307420938);
    }
}
