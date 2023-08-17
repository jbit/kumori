mod resize;

use aws_lambda_events::apigw::{ApiGatewayV2httpRequest, ApiGatewayV2httpResponse};
use aws_lambda_events::encodings::Body;
use aws_lambda_events::http::header::{CONTENT_TYPE, SERVER};
use aws_lambda_events::http::{HeaderMap, HeaderName, HeaderValue};
use lambda_runtime::{service_fn, LambdaEvent};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tracing::debug;

type Result<T> = anyhow::Result<T, lambda_runtime::Error>;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const SERVER_TIMING: HeaderName = HeaderName::from_static("server-timing");
const INVOKE_COUNT: HeaderName = HeaderName::from_static("x-invoke-count");
static INVOKE_COUNTER: AtomicU64 = AtomicU64::new(0);

async fn read_from_bucket(bucket: &str, key: &str) -> Result<Vec<u8>> {
    let config = aws_config::load_from_env().await;
    let s3client = aws_sdk_s3::Client::new(&config);

    debug!("S3://{bucket}/{key} region:{:?}", config.region());

    let command = s3client.get_object().bucket(bucket).key(key);
    let s3object = command.send().await?;
    let mut bytes = vec![];
    let size = s3object
        .body
        .into_async_read()
        .read_to_end(&mut bytes)
        .await?;

    debug!("S3://{bucket}/{key} Read {:.2}KiB", size as f32 / 1024.0);

    Ok(bytes)
}

async fn handler(event: LambdaEvent<ApiGatewayV2httpRequest>) -> Result<ApiGatewayV2httpResponse> {
    let invoke_count = INVOKE_COUNTER.fetch_add(1, SeqCst).to_string();
    debug!("Request: {event:?}");

    let filename = event.payload.path_parameters.get("proxy").unwrap();
    let width_str = event.payload.query_string_parameters.first("width");
    let height_str = event.payload.query_string_parameters.first("height");

    let bucket = std::env::var("KUMORI_BUCKET")?;

    let s3read_start: Instant = Instant::now();
    let original_data = read_from_bucket(&bucket, filename).await?;
    let s3read_dur = s3read_start.elapsed().as_secs_f32() * 1000.0;

    let resize_start: Instant = Instant::now();

    let final_data = if let (Some(width_str), Some(height_str)) = (width_str, height_str) {
        let (width, height) = (width_str.parse().unwrap(), height_str.parse().unwrap());
        resize::resize_jpeg(original_data, width, height)?
    } else {
        original_data
    };
    let resize_dur = resize_start.elapsed().as_secs_f32() * 1000.0;

    let server_timing = format!("s3read;dur={s3read_dur:.3},resize;dur={resize_dur:.3}");

    Ok(ApiGatewayV2httpResponse {
        status_code: 200,
        headers: HeaderMap::from_iter([
            (CONTENT_TYPE, HeaderValue::from_static("image/jpeg")),
            (SERVER, HeaderValue::from_static(PKG_NAME)),
            (SERVER_TIMING, HeaderValue::from_str(&server_timing)?),
            (INVOKE_COUNT, HeaderValue::from_str(&invoke_count)?),
        ]),
        body: Some(Body::Binary(final_data)),
        is_base64_encoded: true,
        ..Default::default()
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .without_time()
        .init();
    lambda_runtime::run(service_fn(handler)).await
}
