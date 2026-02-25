mod redis_pool;
mod modules;
use crate::modules::coordinate_converter_v2::CoordinateConverter;

use redis_pool::RedisPool;
use std::sync::Arc;
use log::info;
use tokio::sync::mpsc;
use crate::modules::message_parser::{parse_message, ParseResult};
use crate::modules::{AprilTagData, ObjectData};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();
    info!("Starting vision_postprocessor...");

    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "192.168.0.71".to_string());
    let redis_port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "26379".to_string());
    let redis_url = format!("redis://{}:{}/0", redis_host, redis_port);
    info!("Redis URL: {}", redis_url);

    let input_stream = std::env::var("REDIS_INPUT_STREAM").unwrap_or_else(|_| "input_stream".to_string());
    let output_stream = std::env::var("REDIS_OUTPUT_STREAM").unwrap_or_else(|_| "output_stream".to_string());
    info!("Input stream: {}, Output stream: {}", input_stream, output_stream);

    let pool = RedisPool::new(&redis_url).await;
    let pool = Arc::new(pool);
    info!("Redis pool initialized");

    let undistortion_config = std::env::var("UNDISTORTION_CONFIG_PATH").unwrap_or_else(|_| "config/camera-configs/undistort_config.json".to_string());
    let homography_config = std::env::var("HOMOGRAPHY_CONFIG_PATH").unwrap_or_else(|_| "config/camera-configs/homography_config.json".to_string());
    info!("Loading CoordinateConverter with undistortion: {}, homography: {}", undistortion_config, homography_config);
    let converter = Arc::new(CoordinateConverter::new(undistortion_config, homography_config).expect("Failed to initialize CoordinateConverter"));
    info!("CoordinateConverter initialized successfully");

    let (tx, mut rx) = mpsc::channel::<String>(2);
    let pool_reader = pool.clone();
    let input_stream_reader = input_stream.clone();
    let mut last_id = String::from("0");

    // Reader task: only reads from Redis and sends to channel
    info!("Starting reader task...");
    let reader_handle = tokio::spawn(async move {
        loop {
            if let Some((msg_id, msg)) =
                pool_reader.read_next_message(&input_stream_reader, last_id.clone()).await {
                last_id = msg_id;
                if tx.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    // Worker task: receives from channel, parses and publishes
    let pool_c = pool.clone();
    let output_stream_c = output_stream.clone();
    let converter_c = converter.clone();
    info!("Starting worker task...");
    let worker_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut pipeline = MessagePipeline::new(msg);
            pipeline
                .parse()
                .coordinate_convert(&converter_c);
            if pipeline.result.success {
                let out_msg = pipeline.to_json();
                pool_c.publish(&output_stream_c, &out_msg).await;
                info!("Published parsed message ({} objects, {} apriltags)", 
                    pipeline.result.object_count, 
                    pipeline.result.apriltag_count);
            }
        }
    });

    let _ = tokio::try_join!(reader_handle, worker_handle);
    info!("Shutting down vision_postprocessor...");
}

// --- Pipeline struct and impl ---
struct MessagePipeline {
    raw: String,
    pub objects: Vec<ObjectData>,
    pub apriltags: Vec<AprilTagData>,
    pub result: ParseResult,
}

impl MessagePipeline {
    fn new(raw: String) -> Self {
        Self {
            raw,
            objects: vec![],
            apriltags: vec![],
            result: ParseResult::default() ,
        }
    }

    fn parse(&mut self) -> &mut Self {
        self.result = parse_message(&self.raw, &mut self.objects, &mut self.apriltags);
        self
    }

    fn coordinate_convert(&mut self, converter: &CoordinateConverter) -> &mut Self {
        converter.convert_batch_objects(&mut self.objects[..self.result.object_count]);
        converter.convert_batch_apriltags(&mut self.apriltags[..self.result.apriltag_count]);
        self
    }

    fn to_json(&self) -> String {
        serde_json::json!({
            "objects": &self.objects[..self.result.object_count],
            "apriltags": &self.apriltags[..self.result.apriltag_count]
        }).to_string()
    }
}

