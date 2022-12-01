use std::sync::Arc;
use serenity::Client;
use serenity::client::bridge::gateway::{ShardMessenger};
use serenity::client::Cache;
use serenity::http::{CacheHttp, Http};
use tokio::sync::Mutex;

lazy_static! {
    static ref CACHE_HTTP_HOLDER: Arc<Mutex<Vec<Arc<CacheAndHttp>>>> = Arc::new(Mutex::new(Vec::new()));
}

#[derive(Debug)]
pub struct CacheAndHttp {
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
    pub shard: Arc<ShardMessenger>
}

impl CacheHttp for CacheAndHttp {
    fn http(&self) -> &Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.cache)
    }
}

impl AsRef<Http> for CacheAndHttp {
    fn as_ref(&self) -> &Http {
        &self.http
    }
}

impl AsRef<Cache> for CacheAndHttp {
    fn as_ref(&self) -> &Cache {
        &self.cache
    }
}

pub async fn register_cache_and_http(cache_http: Arc<CacheAndHttp>) {
    let mut acquire_lock = CACHE_HTTP_HOLDER.lock().await;
    if acquire_lock.is_empty() {
        acquire_lock.push(cache_http);
    }
}

pub async fn get_cache_and_http() -> Arc<CacheAndHttp> {
    let mut acquire_lock = CACHE_HTTP_HOLDER.lock().await;
    acquire_lock.last().expect("Cache and http not registered").clone()
}