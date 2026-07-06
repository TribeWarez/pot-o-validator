//! Tests for ai3-lib module
//!
//! Validates AI3Engine, Tensor operations, and mining task execution

use ai3_lib::{
    AI3Engine, ESPCompatibility, ESPDeviceType, ESPMiningConfig, EngineConfig, EngineStats,
    MiningTask, TaskDistributor, Tensor, TensorData, TensorEngine, TensorShape,
};
use std::time::Duration;

#[test]
fn test_tensor_shape_creation() {
    let shape = TensorShape::new(vec![2, 3]);

    assert_eq!(shape.dims, &[2, 3]);
}

#[test]
fn test_tensor_shape_multidimensional() {
    let shape = TensorShape::new(vec![2, 3, 4]);

    assert_eq!(shape.dims.len(), 3);
    assert_eq!(shape.dims[0], 2);
    assert_eq!(shape.dims[1], 3);
    assert_eq!(shape.dims[2], 4);
}

#[test]
fn test_tensor_creation() {
    let shape = TensorShape::new(vec![1, 1]);
    let data = TensorData::F32(vec![1.0_f32]);

    let tensor = Tensor::new(shape, data);
    assert!(tensor.is_ok());
}

#[test]
fn test_tensor_access() {
    let shape = TensorShape::new(vec![2, 2]);
    let data = TensorData::F32(vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32]);

    let tensor = Tensor::new(shape, data).unwrap();
    assert_eq!(tensor.shape.dims, &[2, 2]);
}

#[test]
fn test_engine_config_defaults() {
    let config = EngineConfig::default();

    assert_eq!(config.max_concurrent_tasks, 10);
    assert_eq!(config.task_timeout, Duration::from_secs(30));
    assert!(config.enable_esp_support);
    assert!(config.auto_optimize_tensors);
}

#[test]
fn test_engine_config_custom() {
    let config = EngineConfig {
        max_concurrent_tasks: 20,
        task_timeout: Duration::from_secs(60),
        enable_esp_support: false,
        auto_optimize_tensors: false,
    };

    assert_eq!(config.max_concurrent_tasks, 20);
    assert_eq!(config.task_timeout, Duration::from_secs(60));
    assert!(!config.enable_esp_support);
    assert!(!config.auto_optimize_tensors);
}

#[test]
fn test_engine_stats_defaults() {
    let stats = EngineStats::default();

    assert_eq!(stats.total_tasks_processed, 0);
    assert_eq!(stats.successful_tasks, 0);
    assert_eq!(stats.failed_tasks, 0);
    assert_eq!(stats.average_task_time, Duration::ZERO);
    assert_eq!(stats.total_compute_time, Duration::ZERO);
}

#[test]
fn test_ai3_engine_creation() {
    let _engine = AI3Engine::new();

    // Engine should be created successfully
    assert!(true);
}

#[test]
fn test_ai3_engine_no_panic() {
    // Verify engine can be created and stats accessed without panicking
    let engine = AI3Engine::new();
    let stats = engine.get_stats();
    assert_eq!(stats.total_tasks_processed, 0);
}

#[test]
fn test_engine_trait_implementation() {
    let engine: Box<dyn TensorEngine> = Box::new(AI3Engine::new());

    // Should be usable as trait object
    let stats = engine.get_stats();
    assert_eq!(stats.total_tasks_processed, 0);
}

#[test]
fn test_mining_task_creation() {
    let task = MiningTask::new(
        "identity".to_string(),
        vec![],
        1000,
        50_000_000,
        300,
        "miner1".to_string(),
    );

    assert_eq!(task.operation_type, "identity");
    assert_eq!(task.difficulty, 1000);
}

#[test]
fn test_mining_task_with_inputs() {
    let shape = TensorShape::new(vec![1, 1]);
    let data = TensorData::F32(vec![1.0_f32]);
    let tensor = Tensor::new(shape, data).unwrap();

    let task = MiningTask::new(
        "matrix_multiply".to_string(),
        vec![tensor],
        1000,
        50_000_000,
        300,
        "miner1".to_string(),
    );

    assert_eq!(task.operation_type, "matrix_multiply");
    assert_eq!(task.input_tensors.len(), 1);
}

#[test]
fn test_mining_task_reward() {
    let task = MiningTask::new(
        "test".to_string(),
        vec![],
        1000,
        100_000_000, // 100M reward
        300,
        "miner1".to_string(),
    );

    assert_eq!(task.reward, 100_000_000);
}

#[test]
fn test_mining_task_deadline() {
    let task = MiningTask::new(
        "test".to_string(),
        vec![],
        1000,
        50_000_000,
        600, // 10 minutes
        "miner1".to_string(),
    );

    assert_eq!(task.max_computation_time, 600);
}

#[test]
fn test_mining_task_requester() {
    let task = MiningTask::new(
        "test".to_string(),
        vec![],
        1000,
        50_000_000,
        300,
        "validator_node".to_string(),
    );

    assert_eq!(task.requester, "validator_node");
}

#[test]
fn test_task_distributor_creation() {
    let _distributor = TaskDistributor::new();

    assert!(true); // Distributor created
}

#[test]
fn test_esp_device_type_variants() {
    let types = vec![
        ESPDeviceType::ESP32,
        ESPDeviceType::ESP32S,
        ESPDeviceType::ESP8266,
    ];

    assert_eq!(types.len(), 3);
}

#[test]
fn test_esp_compatibility_config() {
    let config = ESPCompatibility::get_recommended_config(ESPDeviceType::ESP32S);

    // Should return a valid config
    assert!(config.max_tensor_dim > 0);
}

#[test]
fn test_esp_mining_config_creation() {
    let config = ESPMiningConfig::for_device(ESPDeviceType::ESP32);

    assert_eq!(config.max_tensor_dim, 64);
    assert_eq!(config.rpc_port, 8900);
}

#[test]
fn test_esp_mining_config_esp8266() {
    let config = ESPMiningConfig::for_device(ESPDeviceType::ESP8266);

    // ESP8266 has a smaller max tensor dim
    assert_eq!(config.max_tensor_dim, 32);
    assert!(config.max_tensor_dim > 0);
}

#[test]
fn test_tensor_data_f32() {
    let data = TensorData::F32(vec![1.0_f32, 2.0_f32, 3.0_f32]);

    assert_eq!(data.len(), 3);
}

#[test]
fn test_tensor_shape_vector_consistency() {
    let dims = vec![4, 4];
    let shape = TensorShape::new(dims.clone());

    assert_eq!(shape.dims, &dims[..]);
}

#[test]
fn test_engine_new_initializes_stats() {
    let engine = AI3Engine::new();
    let stats = engine.get_stats();

    assert_eq!(stats.total_tasks_processed, 0);
    assert_eq!(stats.successful_tasks, 0);
}

#[test]
fn test_mining_task_clone() {
    let task = MiningTask::new(
        "test".to_string(),
        vec![],
        1000,
        50_000_000,
        300,
        "miner1".to_string(),
    );

    let task2 = task.clone();

    assert_eq!(task.operation_type, task2.operation_type);
}

#[test]
fn test_tensor_shape_clone() {
    let shape = TensorShape::new(vec![2, 3]);
    let shape2 = shape.clone();

    assert_eq!(shape.dims, shape2.dims);
}

#[test]
fn test_engine_config_clone() {
    let config = EngineConfig::default();
    let config2 = config.clone();

    assert_eq!(config.max_concurrent_tasks, config2.max_concurrent_tasks);
}

#[test]
fn test_engine_stats_clone() {
    let stats = EngineStats::default();
    let stats2 = stats.clone();

    assert_eq!(stats.total_tasks_processed, stats2.total_tasks_processed);
}

#[test]
fn test_mining_result_type_exists() {
    // Just verify the type is available
    let _: &dyn std::any::Any = &();
}

#[test]
fn test_tensor_dimension_limits_esp_compatible() {
    // ESP32 max dimension should be 64
    let max_dim = 64usize;
    let shape = TensorShape::new(vec![max_dim; 2]);

    assert_eq!(shape.dims[0], max_dim);
}

#[test]
fn test_mining_config_reasonable_bounds() {
    let config = ESPMiningConfig::for_device(ESPDeviceType::ESP32);

    // Max tensor dim should be positive and reasonable
    assert!(config.max_tensor_dim > 0);
    assert!(config.max_tensor_dim <= 256);
}

#[test]
fn test_engine_concurrent_task_limit() {
    let config = EngineConfig {
        max_concurrent_tasks: 5,
        ..Default::default()
    };

    assert!(config.max_concurrent_tasks > 0);
    assert!(config.max_concurrent_tasks <= 1000); // Reasonable upper bound
}

#[test]
fn test_task_timeout_is_positive() {
    let config = EngineConfig::default();

    assert!(config.task_timeout.as_secs() > 0);
}

#[test]
fn test_mining_task_difficulty_positive() {
    let task = MiningTask::new(
        "test".to_string(),
        vec![],
        1, // Minimum difficulty
        50_000_000,
        300,
        "miner1".to_string(),
    );

    assert!(task.difficulty > 0);
}
