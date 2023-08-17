use futures::future::join_all;
use reqwest::{self, Url};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env::args, path::Path, time::Instant};
use tokio::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
struct Measurement {
    total_duration: f64,
    invoke_count: u64,
    timings: HashMap<String, f64>,
    size: usize,
}

async fn test(url: &str) -> Measurement {
    let start_time = Instant::now();
    let response = reqwest::get(url).await.unwrap().error_for_status().unwrap();

    let total_duration = start_time.elapsed().as_secs_f64() * 1000.0;

    let invoke_count = response
        .headers()
        .get("x-invoke-count")
        .expect("missing x-invoke-count header")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let timings_str = response
        .headers()
        .get("server-timing")
        .expect("missing server-timing header")
        .to_str()
        .unwrap();

    let mut timings = HashMap::<String, f64>::new();
    for timing in timings_str.split(',') {
        let mut values = timing.split(';');
        let name = values.next().expect("timing lacks a name");
        for value in values {
            let (k, v) = value.split_once('=').expect("timing malformed");
            if k == "dur" {
                timings.insert(name.to_string(), v.parse().unwrap());
            }
        }
    }

    let bytes = response.bytes().await.unwrap();

    Measurement {
        total_duration,
        invoke_count,
        timings,
        size: bytes.len(),
    }
}

#[tokio::main]
async fn main() {
    let mut args = args();
    let prog = args.next().unwrap();
    let [Some(cdk_outputs_file), Some(s3file)] = [args.next(), args.next()] else {
        eprintln!("Usage:\n{prog} cdk-outputs.json ferris.jpg width height");
        return;
    };
    let [width, height] = [args.next(), args.next()];

    let path = Path::new(&cdk_outputs_file)
        .parent()
        .unwrap_or(Path::new("."));

    let cdk_outputs_json = read_to_string(&cdk_outputs_file)
        .await
        .expect(&format!("{cdk_outputs_file}: Failed to read"));

    let cdk_outputs: HashMap<String, HashMap<String, String>> =
        serde_json::from_str(&cdk_outputs_json)
            .expect(&format!("{cdk_outputs_file}: Failed to parse"));

    let endpoints = cdk_outputs
        .get("Kumori")
        .unwrap()
        .iter()
        .filter(|(k, _)| k.starts_with("EndpointUrl"))
        .map(|(_, v)| Url::parse(v).unwrap());

    let mut averages = HashMap::<String, u32>::new();

    for endpoint_url in endpoints {
        let variant = endpoint_url.path_segments().unwrap().next().unwrap();
        let mut url = endpoint_url.join(&s3file).unwrap();
        if let Some(width) = &width {
            url.query_pairs_mut().append_pair("width", width);
        }
        if let Some(height) = &height {
            url.query_pairs_mut().append_pair("height", height);
        }
        eprintln!("{variant:10} URL: {url}");

        let mut results = vec![];
        results.extend(join_all((0..50).map(|_| test(url.as_str()))).await);
        results.extend(join_all((0..50).map(|_| test(url.as_str()))).await);
        results.extend(join_all((0..50).map(|_| test(url.as_str()))).await);

        let results_json = serde_json::to_string_pretty(&results).unwrap();
        let results_path = path.join(format!("{variant}.json"));
        eprintln!("{variant:10} Results: {results_path:?}");
        tokio::fs::write(results_path, results_json).await.unwrap();

        let cold = results.iter().filter(|m| m.invoke_count == 0).count();
        if cold == 0 {
            println!("{variant:10} All warm hits");
        } else {
            println!("{variant:10} {cold} COLD HITS!!!");
        }

        let (min, max, avg) = min_max_avg(&results, |m| m.timings["s3read"]);
        println!("{variant:10} s3read min={min:4.02} max={max:4.02} avg={avg:4.02}");

        let (min, max, avg) = min_max_avg(&results, |m| m.timings["resize"]);
        println!("{variant:10} resize min={min:4.02} max={max:4.02} avg={avg:4.02}");

        let (min, max, avg) = min_max_avg(&results, |m| m.total_duration);
        println!("{variant:10} total  min={min:4.02} max={max:4.02} avg={avg:4.02}");

        averages.insert(variant.to_string(), avg as u32);
    }
}

fn min_max_avg(measurements: &[Measurement], f: impl Fn(&Measurement) -> f64) -> (f64, f64, f64) {
    let min = measurements.iter().fold(f64::MAX, |a, m| a.min(f(m)));
    let max = measurements.iter().fold(f64::MIN, |a, m| a.max(f(m)));
    let avg = measurements.iter().map(f).sum::<f64>() / measurements.len() as f64;

    (min, max, avg)
}
