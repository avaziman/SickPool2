use crypto_bigint::{NonZero, U256};

use super::{hard_config::PPLNS_SHARE_UNITS_256, pplns::Score};

// pub static DIFF1: U256 = U256::ZERO;

// a share with this target will give exactly 1 score point
pub static MAX_TARGET: U256 = DIFF1.saturating_mul(&PPLNS_SHARE_UNITS_256);

static DIFF1: U256 =
    U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

// pub fn get_diff1_score(hash: &U256) -> Score {
//     get_diff_score(hash, &DIFF1)
// }

pub fn get_diff_score(hash: &U256, diff1: &U256) -> Score {
    debug_assert!(hash <= &MAX_TARGET);

    let hash = NonZero::new(*hash).unwrap();
    let (quotient, _remainder) = diff1.saturating_mul(&PPLNS_SHARE_UNITS_256).div_rem(&hash);

    quotient.as_words()[0]
}

pub fn get_target_from_diff_units(diff: u64) -> U256 {
    DIFF1
        .checked_div(&U256::from_u64(diff))
        .unwrap()
        .saturating_mul(&PPLNS_SHARE_UNITS_256)
}

#[cfg(test)]
mod tests {
    use crypto_bigint::U256;

    use crate::p2p::networking::difficulty::{get_diff_score, DIFF1};

    // https://en.bitcoin.it/wiki/Difficulty
    #[test]
    fn bitcoin_diff() {
        let check =
            U256::from_be_hex("00000000000404CB000000000000000000000000000000000000000000000000");

        let result = get_diff_score(&check, &DIFF1);
        assert_eq!(result, 16307420938);
    }
}
