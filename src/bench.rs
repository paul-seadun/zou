use client::{Config, GetResponse};
use hyper::Client;
use hyper::header::{ByteRangeSpec, Headers, Range};
use MirrorsList;
use rayon::prelude::*;
use std::path::Path;
use std::time::{Duration, Instant};
use URL;

/// Number of times to ping the remote server
const PING_TIMES: usize = 5;
/// Number of bytes to download from the remote server
const LEN_BENCH_CHUNK: u64 = 64;

/// Launch a benchmark on a single URL
/// This benchmark tests the network for this URL, downloading five times a 64 bits packet
/// The result is the mean of the five measures
fn launch_bench<'a>(bench_client: &Client, url: URL<'a>) -> u32 {
    let mut c_ping_time: [u32; PING_TIMES] = [0; PING_TIMES];
    for index in 0..PING_TIMES {
        let now = Instant::now();
        let mut header = Headers::new();
        header.set(Range::Bytes(
            vec![ByteRangeSpec::FromTo(0, LEN_BENCH_CHUNK)],
        ));
        match bench_client.get_head_response_using_headers(url, header) {
            Ok(_) => c_ping_time[index] = now.elapsed().subsec_nanos(),
            Err(_) => break,
        }
    }
    // Return 0 if an error occured - the mirror is automatically removed
    if c_ping_time.iter().any(|&x| x == 0) {
        return 0;
    }
    // Return the mean value
    let sum: u32 = c_ping_time.iter().sum();
    sum / PING_TIMES as u32
}

/// Test each URL to download the required file
/// This function returns a list of URLs, which is sorted by median measures (the first URL is the fastest server)
pub fn bench_mirrors<'a>(
    mirrors: MirrorsList<'a>,
    filename: &str,
    ssl_support: bool,
) -> MirrorsList<'a> {
    // Hyper client to make benchmarks
    let current_config = Config { enable_ssl: ssl_support };
    let mut bench_client = current_config.get_hyper_client();
    bench_client.set_read_timeout(Some(Duration::from_secs(3)));
    // Get mirrors list
    // let mut b_mirrors: Vec<(&'a str, u32)> = Vec::with_capacity(PING_TIMES);
    let mut b_mirrors: Vec<(&'a str, u32)> = mirrors
        .par_iter()
        // Launch bench tests
        .map(|mirror| -> (&'a str, u32) {
                 let mirror_file = Path::new(mirror).join(filename);
                 match mirror_file.to_str() {
                     Some(mirror_path) => (mirror, launch_bench(&bench_client, mirror_path)),
                     None => (mirror, 0),
                 }
             })
        // If the bench is equals to 0, an error occured
        .filter(|x| x.1 != 0)
        .collect();
    b_mirrors.sort_by_key(|k| k.1);
    b_mirrors.iter().map(|x| x.0).collect()
}
