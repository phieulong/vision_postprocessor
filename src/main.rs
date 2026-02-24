mod redis_pool;
mod modules;
use crate::modules::coordinate_converter::batch_homography_all_cameras;

use redis_pool::RedisPool;
use std::sync::Arc;
use log::info;
use tokio::sync::mpsc;
use crate::modules::message_parser::{parse_message, ParseResult};
use crate::modules::{AprilTagData, ObjectData};

#[tokio::main]
async fn main() {
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let input_stream = std::env::var("REDIS_INPUT_STREAM").unwrap_or_else(|_| "input_stream".to_string());
    let output_stream = std::env::var("REDIS_OUTPUT_STREAM").unwrap_or_else(|_| "output_stream".to_string());

    let pool = RedisPool::new(&redis_url).await;
    let pool = Arc::new(pool);

    let (tx, mut rx) = mpsc::channel::<String>(2);
    let pool_reader = pool.clone();
    let input_stream_reader = input_stream.clone();
    let mut last_id = String::from("0");

    // Reader task: only reads from Redis and sends to channel
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
    let worker_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut pipeline = MessagePipeline::new(msg);
            pipeline
                .parse()
                .coordinate_convert();
            if pipeline.result.success {
                let out_msg = pipeline.to_json();
                pool_c.publish(&output_stream_c, &out_msg).await;
                info!("Published parsed message");
            }
        }
    });

    let _ = tokio::try_join!(reader_handle, worker_handle);
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

    fn coordinate_convert(&mut self) -> &mut Self {
        // Example: extract matrices and points from objects/apriltags if available
        // This is a placeholder for actual extraction logic
        let matrices = [];
        let points = [];
        let converted = crate::modules::coordinate_converter::batch_homography_all_cameras(&matrices, &points);
        self
    }

    fn to_json(&self) -> String {
        serde_json::json!({
            "objects": &self.objects[..self.result.object_count],
            "apriltags": &self.apriltags[..self.result.apriltag_count]
        }).to_string()
    }
}

