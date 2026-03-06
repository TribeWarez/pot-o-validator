//! Mining task types, results, miner capabilities, and task distribution.

use crate::tensor::Tensor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single mining task (operation, input tensors, difficulty, expiry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningTask {
    /// Unique task id.
    pub id: String,
    /// Operation type (e.g. matrix_multiply, relu).
    pub operation_type: String,
    /// Input tensors for the operation.
    pub input_tensors: Vec<Tensor>,
    /// Difficulty (leading zeros required in hash).
    pub difficulty: u64,
    /// Reward amount.
    pub reward: u64,
    /// Max computation time in seconds.
    pub max_computation_time: u64,
    /// Requester/miner id.
    pub requester: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Expiry timestamp.
    pub expires_at: DateTime<Utc>,
}

impl MiningTask {
    /// Creates a new mining task with generated id and expiry.
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

    /// Returns true if the task has passed its expiry time.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Returns true if the given hash has at least `difficulty` leading zeros.
    pub fn meets_difficulty(&self, hash: &str) -> bool {
        let leading_zeros = self.difficulty as usize;
        hash.chars().take(leading_zeros).all(|c| c == '0')
    }
}

/// Result of a completed mining task (output tensor, nonce, validity).
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

/// Capabilities advertised by a miner (operations, tensor size, ESP flag).
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

/// Aggregate stats for a miner (completed/failed tasks, compute time).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MinerStats {
    /// Number of tasks completed successfully.
    pub tasks_completed: u64,
    /// Number of tasks that failed.
    pub tasks_failed: u64,
    /// Total compute time in ms.
    pub total_compute_time: u64,
    /// Average compute time per task.
    pub average_compute_time: f64,
}

/// Distributes mining tasks to available miners (from .AI3).
#[derive(Debug, Default)]
pub struct TaskDistributor {
    /// Task id -> task.
    pub pending_tasks: HashMap<String, MiningTask>,
}

impl TaskDistributor {
    /// Creates an empty distributor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a task to the pending set.
    pub fn add_task(&mut self, task: MiningTask) {
        self.pending_tasks.insert(task.id.clone(), task);
    }

    /// Returns all pending tasks.
    pub fn get_pending_tasks(&self) -> Vec<&MiningTask> {
        self.pending_tasks.values().collect()
    }

    /// Removes and returns a task by id.
    pub fn remove_task(&mut self, task_id: &str) -> Option<MiningTask> {
        self.pending_tasks.remove(task_id)
    }

    /// Removes expired tasks from the pending set.
    pub fn cleanup_expired_tasks(&mut self) {
        self.pending_tasks.retain(|_, task| !task.is_expired());
    }
}
