use crypto_bigint::{CheckedMul, NonZero, U256, U512};

use super::{
    hard_config::{PPLNS_DIFF_MULTIPLIER, PPLNS_SHARE_UNITS, PPLNS_SHARE_UNITS_256},
    pplns::Score,
};

// a share with this target will give exactly 1 score point
pub static MAX_TARGET: U256 = U256::MAX.wrapping_div(&PPLNS_SHARE_UNITS_256);

// pub fn get_diff1_score(hash: &U256) -> Score {
//     get_diff_score(hash, &DIFF1)
// }

pub fn get_diff_score(hash: &U256, diff1: &U256) -> Score {
    // debug_assert!(diff1 <= &MAX_TARGET);
    // assumes that target is PPLNS_SHARE_UNITS times smaller than U256::MAX

    // let hash = NonZero::new(*hash).unwrap();
    let (lo, high) = diff1.mul_wide(&PPLNS_SHARE_UNITS_256); //.unwrap().div_rem(&hash);
    let diff1_512 = U512::from((lo, high));
    let (quotient, _) =
        diff1_512.div_rem(&NonZero::new(U512::from((hash.clone(), U256::ZERO))).unwrap());

    std::cmp::min(
        quotient.as_words()[0],
        PPLNS_DIFF_MULTIPLIER * PPLNS_SHARE_UNITS,
    )
}

pub fn get_target_from_diff_units(diff_millis: u64, diff1: &U256) -> U256 {
    diff1
        .checked_mul(&PPLNS_SHARE_UNITS_256)
        .unwrap()
        .wrapping_div(&U256::from_u64(diff_millis))
}

#[cfg(test)]
mod tests {
    use crypto_bigint::U256;

    use crate::p2p::networking::difficulty::get_diff_score;

    pub static DIFF1: U256 =
        U256::from_be_hex("00000000FFFF0000000000000000000000000000000000000000000000000000");

    // https://en.bitcoin.it/wiki/Difficulty
    // block 40k
    #[test]
    fn bitcoin_diff() {
        // target is 000000008cc30000000000000000000000000000000000000000000000000000
        let check =
            U256::from_be_hex("000000008cc30000000000000000000000000000000000000000000000000000");

        let result = get_diff_score(&check, &DIFF1);
        assert_eq!(result, 1818648 /* 536145414 */);
    }
}
