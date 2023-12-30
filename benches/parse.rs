use crypto_bigint::U256;

use criterion::{criterion_group, criterion_main, Criterion};
use sickpool2lib::{
    p2p::networking::difficulty::get_diff_score,
    protocol::JsonRpcProtocol,
    stratum::stratum_v1::StratumV1, coins::{bitcoin::Btc, coin::Coin},
};

extern crate sickpool2lib;
// use SickPool2::stratum::stratum_server::;

fn criterion_benchmark(c: &mut Criterion) {
    let req = r#"{"params": ["slush.miner1", "000000bf", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}"#;

    c.bench_function("parse", move |b| {
        b.iter(|| {
            let json_req = JsonRpcProtocol::<StratumV1>::parse_request(&req.as_bytes()).unwrap();

            StratumV1::parse_stratum_req(json_req.method, json_req.params).unwrap();
        })
    });
}

fn criterion_benchmark2(c: &mut Criterion) {
    let mut check =
        U256::from_be_hex("00000000000404CB000000000000000000000000000000000000000000000000");

    c.bench_function("getdiff", move |b| {
        b.iter(|| {
            get_diff_score(&check, &Btc::DIFF1);
            check = check.wrapping_add(&U256::ONE);
        })
    });
}

// fn bench_job_fetch(c: &mut Criterion) {
//     let test_cli = bitcoincore_rpc::Client::new(
//         "127.0.0.1:34254",
//         Auth::CookieFile(PathBuf::from("/home/sickguy/.bitcoin/regtest/.cookie")),
//     )
//     .unwrap();

//     c.bench_function("parse", |b| b.iter(|| test_cli.fetch_header()));
// }

criterion_group!(
    benches,
    criterion_benchmark,
    criterion_benchmark2 /* , bench_job_fetch */
);
criterion_main!(benches);
