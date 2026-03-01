pub mod tensor;
pub mod operations;
pub mod mining;
pub mod esp_compat;

pub use tensor::{Tensor, TensorData, TensorShape};
pub use operations::{ActivationFunction, Convolution, MatrixMultiply, TensorOp, VectorOp};
pub use mining::{MinerCapabilities, MinerStats, MiningResult, MiningTask, TaskDistributor};
pub use esp_compat::{ESPCompatibility, ESPDeviceType, ESPMiningConfig};

use pot_o_core::TribeResult;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Main AI3 Engine -- coordinates tensor operations and mining tasks.
/// Ported from .AI3 ai3-lib with PoT-O extensions.
pub struct AI3Engine {
    pub task_distributor: TaskDistributor,
    performance_stats: Arc<Mutex<EngineStats>>,
    config: EngineConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub max_concurrent_tasks: usize,
    pub task_timeout: Duration,
    pub enable_esp_support: bool,
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

    pub fn get_stats(&self) -> EngineStats {
        self.performance_stats
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

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

impl Default for AI3Engine {
    fn default() -> Self {
        Self::new()
    }
}
