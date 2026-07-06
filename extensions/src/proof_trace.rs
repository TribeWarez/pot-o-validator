use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofTrace {
    pub challenge_id: String,
    pub miner_pubkey: String,
    pub device_type: String,
    pub accepted: bool,
    pub path_distance: u32,
    pub mml_score: f64,
    pub neural_paths_tested: usize,
    pub successful_paths: usize,
    pub failed_paths: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct ProofTraceStore {
    traces: Mutex<VecDeque<ProofTrace>>,
    max_size: usize,
}

impl ProofTraceStore {
    pub fn new(max_size: usize) -> Self {
        Self {
            traces: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
        }
    }

    pub fn record(&self, trace: ProofTrace) {
        let mut traces = self.traces.lock().unwrap();
        if traces.len() >= self.max_size {
            traces.pop_front();
        }
        traces.push_back(trace);
    }

    pub fn recent(&self, limit: usize) -> Vec<ProofTrace> {
        let traces = self.traces.lock().unwrap();
        traces.iter().rev().take(limit).cloned().collect()
    }
}
