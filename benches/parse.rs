use sickpool2lib::{
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
            let json_req = JsonRpcProtocol::<StratumV1::<bitcoincore_rpc::Client>, StratumV1ErrorCodes>::parse_request(&req).unwrap();

            let stratum_req = StratumV1::<bitcoincore_rpc::Client>::parse_stratum_req(json_req.method, json_req.params).unwrap();
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

criterion_group!(benches, criterion_benchmark /* , bench_job_fetch */);
criterion_main!(benches);
