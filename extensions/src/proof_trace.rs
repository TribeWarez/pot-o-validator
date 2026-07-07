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
    path: Mutex<Option<String>>,
}

impl Clone for ProofTraceStore {
    fn clone(&self) -> Self {
        let traces = self.traces.lock().unwrap();
        let path = self.path.lock().unwrap();
        let new = ProofTraceStore {
            traces: Mutex::new(traces.clone()),
            max_size: self.max_size,
            path: Mutex::new(path.clone()),
        };
        drop(traces);
        drop(path);
        new
    }
}

impl ProofTraceStore {
    pub fn new(max_size: usize) -> Self {
        Self {
            traces: Mutex::new(VecDeque::with_capacity(max_size)),
            max_size,
            path: Mutex::new(None),
        }
    }

    pub fn set_path(&self, path: &str) {
        *self.path.lock().unwrap() = Some(path.to_string());
    }

    pub fn load_from_file(&self) {
        let path = self.path.lock().unwrap();
        let path = match path.as_ref() {
            Some(p) => p.clone(),
            None => return,
        };
        drop(path);
        let path = self.path.lock().unwrap().clone().unwrap();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(traces) = serde_json::from_str::<Vec<ProofTrace>>(&content) {
                let mut t = self.traces.lock().unwrap();
                t.clear();
                for trace in traces.into_iter().rev().take(self.max_size).rev() {
                    if t.len() >= self.max_size {
                        t.pop_front();
                    }
                    t.push_back(trace);
                }
            }
        }
    }

    pub fn save_to_file(&self) -> Result<(), String> {
        let path = self.path.lock().unwrap().clone();
        let path = match path {
            Some(p) => p,
            None => return Ok(()),
        };
        let traces = self.traces.lock().unwrap();
        let all: Vec<ProofTrace> = traces.iter().cloned().collect();
        let json = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
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
