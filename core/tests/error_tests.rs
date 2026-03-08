//! Tests for pot-o-core error handling and TribeError types
//!
//! Validates error creation, formatting, and trait implementations

use pot_o_core::{TribeError, TribeResult};
use std::io;

#[test]
fn test_error_invalid_operation_creation() {
    let msg = "operation failed";
    let err = TribeError::InvalidOperation(msg.to_string());
    assert_eq!(err.to_string(), "Invalid operation: operation failed");
}

#[test]
fn test_error_proof_validation_failed() {
    let msg = "signature mismatch";
    let err = TribeError::ProofValidationFailed(msg.to_string());
    assert_eq!(
        err.to_string(),
        "Proof validation failed: signature mismatch"
    );
}

#[test]
fn test_error_tensor_error_creation() {
    let msg = "shape mismatch";
    let err = TribeError::TensorError(msg.to_string());
    assert_eq!(err.to_string(), "Tensor operation error: shape mismatch");
}

#[test]
fn test_error_tensor_network_error() {
    let msg = "vertex limit exceeded";
    let err = TribeError::TensorNetworkError(msg.to_string());
    assert_eq!(
        err.to_string(),
        "Tensor network error: vertex limit exceeded"
    );
}

#[test]
fn test_error_tensor_network_full() {
    let err = TribeError::TensorNetworkFull;
    assert_eq!(
        err.to_string(),
        "Tensor network is full: cannot add more vertices/edges"
    );
}

#[test]
fn test_error_chain_bridge_error() {
    let msg = "bridge not available";
    let err = TribeError::ChainBridgeError(msg.to_string());
    assert_eq!(err.to_string(), "Chain bridge error: bridge not available");
}

#[test]
fn test_error_network_error() {
    let msg = "connection timeout";
    let err = TribeError::NetworkError(msg.to_string());
    assert_eq!(err.to_string(), "Network error: connection timeout");
}

#[test]
fn test_error_config_error() {
    let msg = "missing api_key";
    let err = TribeError::ConfigError(msg.to_string());
    assert_eq!(err.to_string(), "Configuration error: missing api_key");
}

#[test]
fn test_error_device_error() {
    let msg = "ESP32 connection lost";
    let err = TribeError::DeviceError(msg.to_string());
    assert_eq!(
        err.to_string(),
        "Device protocol error: ESP32 connection lost"
    );
}

#[test]
fn test_error_serialization_error() {
    let msg = "json parsing failed";
    let err = TribeError::SerializationError(msg.to_string());
    assert_eq!(err.to_string(), "Serialization error: json parsing failed");
}

#[test]
fn test_error_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let tribe_err: TribeError = io_err.into();
    assert!(tribe_err.to_string().contains("file not found"));
}

#[test]
fn test_tribe_result_ok() {
    let result: TribeResult<i32> = Ok(42);
    assert_eq!(result, Ok(42));
}

#[test]
fn test_tribe_result_error() {
    let result: TribeResult<i32> = Err(TribeError::InvalidOperation("test".to_string()));
    assert!(result.is_err());
}

#[test]
fn test_error_debug_impl() {
    let err = TribeError::InvalidOperation("debug test".to_string());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("InvalidOperation"));
    assert!(debug_str.contains("debug test"));
}

#[test]
fn test_error_clone() {
    let err1 = TribeError::ProofValidationFailed("test".to_string());
    let err2 = err1.clone();
    assert_eq!(err1.to_string(), err2.to_string());
}

#[test]
fn test_multiple_error_variants_display() {
    let errors = vec![
        TribeError::InvalidOperation("op".to_string()),
        TribeError::ProofValidationFailed("proof".to_string()),
        TribeError::TensorError("tensor".to_string()),
        TribeError::TensorNetworkError("network".to_string()),
        TribeError::TensorNetworkFull,
        TribeError::ChainBridgeError("bridge".to_string()),
        TribeError::NetworkError("net".to_string()),
        TribeError::ConfigError("config".to_string()),
        TribeError::DeviceError("device".to_string()),
        TribeError::SerializationError("serial".to_string()),
    ];

    for err in errors {
        // Ensure all error types have non-empty Display output
        assert!(!err.to_string().is_empty());
    }
}

#[test]
fn test_error_equality() {
    let err1 = TribeError::InvalidOperation("test".to_string());
    let err2 = TribeError::InvalidOperation("test".to_string());
    // Note: Debug impl shows these are equal as types (though Error trait doesn't implement PartialEq)
    assert_eq!(format!("{:?}", err1), format!("{:?}", err2));
}

#[test]
fn test_tensor_network_error_variants() {
    // Test capacity error
    let full_err = TribeError::TensorNetworkFull;
    assert!(full_err.to_string().contains("full"));

    // Test generic tensor network error
    let generic_err = TribeError::TensorNetworkError("specific issue".to_string());
    assert!(generic_err.to_string().contains("specific issue"));
}

#[test]
fn test_error_result_conversions() {
    fn returns_tribe_result() -> TribeResult<String> {
        Err(TribeError::ConfigError("missing field".to_string()))
    }

    let result = returns_tribe_result();
    assert!(result.is_err());
    match result {
        Err(TribeError::ConfigError(msg)) => assert_eq!(msg, "missing field"),
        _ => panic!("Expected ConfigError"),
    }
}

#[test]
fn test_io_error_integration() {
    // Test that IO errors convert properly
    let io_result: Result<(), io::Error> = Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "access denied",
    ));

    let tribe_result: TribeResult<()> = io_result.map_err(|e| e.into());
    assert!(tribe_result.is_err());
}

/// Test that error messages are informative and non-empty
#[test]
fn test_all_error_messages_informative() {
    let test_cases = vec![
        (
            TribeError::InvalidOperation("".to_string()),
            "InvalidOperation should format",
        ),
        (
            TribeError::ProofValidationFailed("".to_string()),
            "ProofValidationFailed should format",
        ),
        (
            TribeError::TensorError("".to_string()),
            "TensorError should format",
        ),
        (
            TribeError::ChainBridgeError("".to_string()),
            "ChainBridgeError should format",
        ),
        (
            TribeError::NetworkError("".to_string()),
            "NetworkError should format",
        ),
        (
            TribeError::ConfigError("".to_string()),
            "ConfigError should format",
        ),
        (
            TribeError::DeviceError("".to_string()),
            "DeviceError should format",
        ),
        (
            TribeError::SerializationError("".to_string()),
            "SerializationError should format",
        ),
        (
            TribeError::TensorNetworkError("".to_string()),
            "TensorNetworkError should format",
        ),
    ];

    for (err, _msg) in test_cases {
        // All errors should have a display message
        let display = err.to_string();
        assert!(!display.is_empty(), "Error message should not be empty");
        assert!(display.len() > 3, "Error message should be descriptive");
    }
}
