use std::{time::Duration};

use crypto_bigint::{CheckedMul, U256};
use log::{info, warn};

use crate::{p2p::networking::{hard_config::MAX_RETARGET_FACTOR, difficulty::{MAX_TARGET, get_diff1_score}}, stratum::header::BlockHeader};

use super::{block::Block};

struct Adjustment {
    time: u32,
    height: u32,
    target: U256,
}

pub struct TargetManager {
    last_adjustment: Adjustment,
    target_time: Duration,
    // once in per how many mainnet blocks to readjust pool diff.
    diff_adjust_blocks: u32,
}

// start each pool difficulty with the genesis block difficulty
// TODO: save pool start time and block maybe
impl TargetManager {
    pub fn new<T: Block>(target_time: Duration, diff_adjust: u32) -> Self {
        let _genesis = T::genesis();
        // let target = genesis.get_header().get_target();
        let target = MAX_TARGET;

        info!("Initial p2p target: {}, difficulty: {}", target, get_diff1_score(&target));

        info!("MAX TARGET: {}", MAX_TARGET);

        Self {
            last_adjustment: Adjustment {
                time: 1687522225,
                target,
                height: 0,
            },
            target_time,
            diff_adjust_blocks: diff_adjust,
        }
    }

    pub fn target(&self) -> &U256 {
        &self.last_adjustment.target
    }

    pub fn adjust(&mut self, current_height: u32, block: &impl Block) {
        if current_height - self.last_adjustment.height < self.diff_adjust_blocks {
            return;
        }

        let current_time = block.get_header().get_time();
        let current_target = self.last_adjustment.target;
        let passed_secs =
            std::cmp::max(1, current_time as i64 - self.last_adjustment.time as i64) as u64;

        info!("Current target: {}", current_target);

        let mut passed_ms = passed_secs * 1000;
        let expected_ms = self.target_time.as_millis() as u64 * self.diff_adjust_blocks as u64;

        if passed_ms < expected_ms / MAX_RETARGET_FACTOR {
            passed_ms = expected_ms / MAX_RETARGET_FACTOR;
        }
        if passed_ms > expected_ms * MAX_RETARGET_FACTOR {
            passed_ms = expected_ms * MAX_RETARGET_FACTOR;
        }

        info!("Passed: {}, Expected {} (ms)", passed_ms, expected_ms);

        // if difficulty too low keep it
        let mut new_target = current_target
            .checked_div(&U256::from(expected_ms))
            .unwrap_or(current_target);

        if new_target == current_target {
            warn!("Failed to retarget!");
        } else {
            new_target = new_target
                .checked_mul(&U256::from(passed_ms))
                .unwrap_or(current_target);
        }
        info!("New target: {}, time: {}", new_target, current_time);

        self.last_adjustment = Adjustment {
            time: current_time,
            height: current_height,
            target: new_target,
        }
    }
}
