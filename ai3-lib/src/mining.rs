use crate::tensor::Tensor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningTask {
    pub id: String,
    pub operation_type: String,
    pub input_tensors: Vec<Tensor>,
    pub difficulty: u64,
    pub reward: u64,
    pub max_computation_time: u64,
    pub requester: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl MiningTask {
    pub fn new(
        operation_type: String,
        input_tensors: Vec<Tensor>,
        difficulty: u64,
        reward: u64,
        max_computation_time: u64,
        requester: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operation_type,
            input_tensors,
            difficulty,
            reward,
            max_computation_time,
            requester,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(max_computation_time as i64),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn meets_difficulty(&self, hash: &str) -> bool {
        let leading_zeros = self.difficulty as usize;
        hash.chars().take(leading_zeros).all(|c| c == '0')
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningResult {
    pub task_id: String,
    pub miner_id: String,
    pub nonce: u64,
    pub hash: String,
    pub output_tensor: Tensor,
    pub computation_time: u64,
    pub timestamp: DateTime<Utc>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerCapabilities {
    pub supported_operations: Vec<String>,
    pub max_tensor_size: usize,
    pub is_esp_device: bool,
    pub max_computation_time: u64,
}

impl Default for MinerCapabilities {
    fn default() -> Self {
        Self {
            supported_operations: vec![
                "matrix_multiply".into(),
                "convolution".into(),
                "relu".into(),
                "sigmoid".into(),
                "tanh".into(),
                "dot_product".into(),
                "normalize".into(),
            ],
            max_tensor_size: 64 * 64 * 4, // 64x64 f32
            is_esp_device: false,
            max_computation_time: 300,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MinerStats {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_compute_time: u64,
    pub average_compute_time: f64,
}

/// Distributes mining tasks to available miners (from .AI3)
#[derive(Debug, Default)]
pub struct TaskDistributor {
    pub pending_tasks: HashMap<String, MiningTask>,
}

impl TaskDistributor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_task(&mut self, task: MiningTask) {
        self.pending_tasks.insert(task.id.clone(), task);
    }

    pub fn get_pending_tasks(&self) -> Vec<&MiningTask> {
        self.pending_tasks.values().collect()
    }

    pub fn remove_task(&mut self, task_id: &str) -> Option<MiningTask> {
        self.pending_tasks.remove(task_id)
    }

    pub fn cleanup_expired_tasks(&mut self) {
        self.pending_tasks.retain(|_, task| !task.is_expired());
    }
}
