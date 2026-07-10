use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use tokio::sync::Mutex;

struct RateLimiterState {
    windows: HashMap<IpAddr, VecDeque<Instant>>,
}

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                windows: HashMap::new(),
            })),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let cutoff = now - self.window;

        let entry = state.windows.entry(ip).or_insert_with(VecDeque::new);

        while entry.front().is_some_and(|t| *t < cutoff) {
            entry.pop_front();
        }

        if entry.len() >= self.max_requests {
            return false;
        }

        entry.push_back(now);
        true
    }
}

pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::extract::Extension(limiter): axum::extract::Extension<RateLimiter>,
    request: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let ip = addr.ip();
    if limiter.check(ip).await {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(serde_json::json!({ "error": "rate limit exceeded" })),
        )
            .into_response()
    }
}
