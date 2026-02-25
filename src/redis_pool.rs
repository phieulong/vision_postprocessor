use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::sync::Arc;
use log::info;

#[derive(Clone)]
pub struct RedisPool {
    pub pool: Arc<Pool>,
}

impl RedisPool {
    pub async fn new(url: &str) -> Self {
        info!("Initializing Redis pool with URL: {}", url);
        let cfg = Config::from_url(url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).expect("Failed to create Redis pool");
        info!("Redis pool created successfully");
        Self { pool: Arc::new(pool) }
    }

    pub async fn get_conn(&self) -> deadpool_redis::Connection {
        self.pool.get().await.unwrap()
    }

    pub async fn read_next_message(&self, stream_key: &str, last_id: String) -> Option<(String, String)> {
        let mut conn = self.get_conn().await;

        let result: redis::RedisResult<Vec<(String, Vec<(String, Vec<(String, String)>)>)>> =
            redis::cmd("XREAD")
                .arg("BLOCK")
                .arg(0)
                .arg("STREAMS")
                .arg(stream_key)
                .arg(last_id.clone())
                .query_async(&mut conn)
                .await;

        if let Ok(entries) = result {
            for (_stream, messages) in entries {
                for (message_id, fields) in messages {
                    for (field, value) in fields {
                        if field == "metadata" {
                            return Some((message_id, value));
                        }
                    }
                }
            }
        }

        None
    }

    pub async fn publish(&self, stream_key: &str, message: &str) -> bool {
        let mut conn = self.get_conn().await;
        let result: redis::RedisResult<String> = redis::cmd("XADD")
            .arg(stream_key)
            .arg("*")
            .arg("metadata")
            .arg(message)
            .query_async(&mut conn)
            .await;
        result.is_ok()
    }
}
