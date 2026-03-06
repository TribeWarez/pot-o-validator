//! AI3 support library: tensor engine, ESP compatibility, and mining task execution for PoT-O.

pub mod esp_compat;
pub mod mining;
pub mod operations;
pub mod tensor;

pub use esp_compat::{ESPCompatibility, ESPDeviceType, ESPMiningConfig};
pub use mining::{MinerCapabilities, MinerStats, MiningResult, MiningTask, TaskDistributor};
pub use operations::{ActivationFunction, Convolution, MatrixMultiply, TensorOp, VectorOp};
pub use tensor::{Tensor, TensorData, TensorShape};

use pot_o_core::TribeResult;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Main AI3 engine: coordinates tensor operations and mining tasks (Ported from .AI3 ai3-lib with PoT-O extensions).
pub struct AI3Engine {
    /// Distributor for mining tasks.
    pub task_distributor: TaskDistributor,
    performance_stats: Arc<Mutex<EngineStats>>,
    config: EngineConfig,
}

/// Configuration for the AI3 engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum concurrent tasks.
    pub max_concurrent_tasks: usize,
    /// Per-task timeout.
    pub task_timeout: Duration,
    /// Whether to enable ESP-sized tensor clamping.
    pub enable_esp_support: bool,
    /// Whether to auto-optimize tensor dimensions.
    pub auto_optimize_tensors: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            task_timeout: Duration::from_secs(30),
            enable_esp_support: true,
            auto_optimize_tensors: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineStats {
    pub total_tasks_processed: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub average_task_time: Duration,
    pub total_compute_time: Duration,
    pub start_time: Instant,
}

impl Default for EngineStats {
    fn default() -> Self {
        Self {
            total_tasks_processed: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_task_time: Duration::ZERO,
            total_compute_time: Duration::ZERO,
            start_time: Instant::now(),
        }
    }
}

/// Abstraction over the tensor execution engine so callers can depend on a trait
/// (e.g. for testing or alternate backends) instead of a concrete struct.
pub trait TensorEngine: Send + Sync {
    fn execute_task(&self, task: &MiningTask) -> TribeResult<Tensor>;
    fn get_stats(&self) -> EngineStats;
    fn record_result(&self, success: bool, duration: Duration);
}

impl AI3Engine {
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            task_distributor: TaskDistributor::new(),
            performance_stats: Arc::new(Mutex::new(EngineStats::default())),
            config,
        }
    }

    /// Execute a tensor operation from a mining task and return the result tensor.
    pub fn execute_task(&self, task: &MiningTask) -> TribeResult<Tensor> {
        let op = operations::parse_operation(&task.operation_type)?;
        let input = task
            .input_tensors
            .first()
            .ok_or_else(|| pot_o_core::TribeError::TensorError("No input tensors".into()))?;

        let result = op.execute(input)?;

        if self.config.auto_optimize_tensors && self.config.enable_esp_support {
            let max_dim = pot_o_core::ESP_MAX_TENSOR_DIM;
            Ok(result.clamp_dimensions(max_dim))
        } else {
            Ok(result)
        }
    }

    /// Returns current engine statistics.
    pub fn get_stats(&self) -> EngineStats {
        self.performance_stats
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// Records a task result for statistics.
    pub fn record_result(&self, success: bool, duration: Duration) {
        if let Ok(mut stats) = self.performance_stats.lock() {
            stats.total_tasks_processed += 1;
            if success {
                stats.successful_tasks += 1;
            } else {
                stats.failed_tasks += 1;
            }
            stats.total_compute_time += duration;
            if stats.total_tasks_processed > 0 {
                stats.average_task_time =
                    stats.total_compute_time / stats.total_tasks_processed as u32;
            }
        }
    }
}

impl TensorEngine for AI3Engine {
    fn execute_task(&self, task: &MiningTask) -> TribeResult<Tensor> {
        <AI3Engine>::execute_task(self, task)
    }

    fn get_stats(&self) -> EngineStats {
        <AI3Engine>::get_stats(self)
    }

    fn record_result(&self, success: bool, duration: Duration) {
        <AI3Engine>::record_result(self, success, duration)
    }
}

impl Default for AI3Engine {
    fn default() -> Self {
        Self::new()
    }
}
