use crypto_bigint::U256;
use sickpool2lib::{
    p2p::networking::difficulty::get_diff,
    protocol::JsonRpcProtocol,
    stratum::{protocol::StratumV1ErrorCodes, stratum_v1::StratumV1},
};

use criterion::{criterion_group, criterion_main, Criterion};

extern crate sickpool2lib;
// use SickPool2::stratum::stratum_server::;

fn criterion_benchmark(c: &mut Criterion) {
    let reqs = r#"{"params": ["slush.miner1", "000000bf", "00000001", "504e86ed", "b2957c02"], "id": 4, "method": "mining.submit"}"#;
    let req = String::from(reqs);

    c.bench_function("parse", move |b| {
        b.iter(|| {
            let json_req = JsonRpcProtocol::<StratumV1<bitcoincore_rpc::Client>>::parse_request(
                &req.as_bytes(),
            )
            .unwrap();

            StratumV1::<bitcoincore_rpc::Client>::parse_stratum_req(
                json_req.method,
                json_req.params,
            )
            .unwrap();
        })
    });
}

fn criterion_benchmark2(c: &mut Criterion) {
    let mut check =
        U256::from_be_hex("00000000000404CB000000000000000000000000000000000000000000000000");

    c.bench_function("getdiff", move |b| {
        b.iter(|| {
            get_diff(check);
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

criterion_group!(benches, criterion_benchmark, criterion_benchmark2 /* , bench_job_fetch */);
criterion_main!(benches);
