use std::collections::HashSet;

use crypto_bigint::U256;

#[derive(Debug, Default)]
pub struct DuplicateHashChecker(HashSet<u64>);

impl DuplicateHashChecker {
    fn short_hash(hash: &U256) -> u64 {
        hash.as_words()[0]
    }

    pub fn did_contain(&mut self, hash: &U256) -> bool {
        let short = Self::short_hash(hash);
        let res = self.0.contains(&short);

        if !res {
            self.0.insert(short);
        }

        res
    }
}