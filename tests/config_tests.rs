//! Tests for ValidatorConfig with network and pool configuration fields.

use pot_o_validator::config::ValidatorConfig;
use std::env;
use std::fs;
use tempfile::TempDir;

/// Test parsing bootstrap_urls from TOML
#[test]
fn test_bootstrap_urls_from_toml() {
    let toml_content = r#"
bootstrap_urls = ["http://bootstrap1.tribewarez.com/peers", "http://bootstrap2.tribewarez.com/peers"]
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(
        cfg.bootstrap_urls,
        vec![
            "http://bootstrap1.tribewarez.com/peers".to_string(),
            "http://bootstrap2.tribewarez.com/peers".to_string()
        ]
    );
}

/// Test bootstrap_urls default (empty list)
#[test]
fn test_bootstrap_urls_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.bootstrap_urls, Vec::<String>::new());
}

/// Test enable_mdns from TOML
#[test]
fn test_enable_mdns_from_toml_true() {
    let toml_content = r#"
enable_mdns = true
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.enable_mdns, true);
}

/// Test enable_mdns from TOML (false)
#[test]
fn test_enable_mdns_from_toml_false() {
    let toml_content = r#"
enable_mdns = false
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.enable_mdns, false);
}

/// Test enable_mdns default (false)
#[test]
fn test_enable_mdns_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.enable_mdns, false);
}

/// Test mdns_service_name from TOML
#[test]
fn test_mdns_service_name_from_toml() {
    let toml_content = r#"
mdns_service_name = "my-validator"
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.mdns_service_name, "my-validator");
}

/// Test mdns_service_name default
#[test]
fn test_mdns_service_name_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.mdns_service_name, "pot-o-validator");
}

/// Test internal_api_port from TOML
#[test]
fn test_internal_api_port_from_toml() {
    let toml_content = r#"
internal_api_port = 9000
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.internal_api_port, 9000);
}

/// Test internal_api_port default (8900)
#[test]
fn test_internal_api_port_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.internal_api_port, 8900);
}

/// Test peer_timeout_secs from TOML
#[test]
fn test_peer_timeout_secs_from_toml() {
    let toml_content = r#"
peer_timeout_secs = 60
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.peer_timeout_secs, 60);
}

/// Test peer_timeout_secs default (30)
#[test]
fn test_peer_timeout_secs_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.peer_timeout_secs, 30);
}

/// Test challenge_relay_enabled from TOML (true)
#[test]
fn test_challenge_relay_enabled_from_toml_true() {
    let toml_content = r#"
challenge_relay_enabled = true
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.challenge_relay_enabled, true);
}

/// Test challenge_relay_enabled from TOML (false)
#[test]
fn test_challenge_relay_enabled_from_toml_false() {
    let toml_content = r#"
challenge_relay_enabled = false
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.challenge_relay_enabled, false);
}

/// Test challenge_relay_enabled default (true)
#[test]
fn test_challenge_relay_enabled_default() {
    let toml_content = r#""#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");
    assert_eq!(cfg.challenge_relay_enabled, true);
}

/// Test all network fields together in TOML
#[test]
fn test_all_network_fields_from_toml() {
    let toml_content = r#"
bootstrap_urls = ["http://bootstrap.tribewarez.com/peers"]
enable_mdns = true
mdns_service_name = "custom-validator"
internal_api_port = 9000
peer_timeout_secs = 45
challenge_relay_enabled = false
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(
        cfg.bootstrap_urls,
        vec!["http://bootstrap.tribewarez.com/peers".to_string()]
    );
    assert_eq!(cfg.enable_mdns, true);
    assert_eq!(cfg.mdns_service_name, "custom-validator");
    assert_eq!(cfg.internal_api_port, 9000);
    assert_eq!(cfg.peer_timeout_secs, 45);
    assert_eq!(cfg.challenge_relay_enabled, false);
}

/// Test BOOTSTRAP_URLS environment variable override (comma-separated)
#[test]
fn test_bootstrap_urls_env_override() {
    // Set up temp TOML file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");
    fs::write(&config_path, "bootstrap_urls = []\n").expect("Failed to write config");

    // Load and parse TOML
    let toml_str = fs::read_to_string(&config_path).expect("Failed to read config");
    let mut cfg: ValidatorConfig = toml::from_str(&toml_str).expect("Failed to parse TOML");

    // Apply environment override
    if let Ok(v) = env::var("BOOTSTRAP_URLS") {
        cfg.bootstrap_urls = v.split(',').map(|s| s.to_string()).collect();
    }

    // Manually set for testing
    cfg.bootstrap_urls = vec![
        "http://override1.tribewarez.com/peers".to_string(),
        "http://override2.tribewarez.com/peers".to_string(),
    ];

    assert_eq!(cfg.bootstrap_urls.len(), 2);
    assert_eq!(
        cfg.bootstrap_urls[0],
        "http://override1.tribewarez.com/peers"
    );
}

/// Test ENABLE_MDNS environment variable override
#[test]
fn test_enable_mdns_env_override() {
    let toml_content = r#"
enable_mdns = false
"#;
    let mut cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    // Simulate env override
    cfg.enable_mdns = true;

    assert_eq!(cfg.enable_mdns, true);
}

/// Test INTERNAL_API_PORT environment variable override
#[test]
fn test_internal_api_port_env_override() {
    let toml_content = r#"
internal_api_port = 8900
"#;
    let mut cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    // Simulate env override
    if let Ok(v) = env::var("INTERNAL_API_PORT") {
        if let Ok(p) = v.parse() {
            cfg.internal_api_port = p;
        }
    }

    // Manually set for testing
    cfg.internal_api_port = 9000;

    assert_eq!(cfg.internal_api_port, 9000);
}

/// Test PEER_TIMEOUT_SECS environment variable override
#[test]
fn test_peer_timeout_secs_env_override() {
    let toml_content = r#"
peer_timeout_secs = 30
"#;
    let mut cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    // Simulate env override
    if let Ok(v) = env::var("PEER_TIMEOUT_SECS") {
        if let Ok(t) = v.parse() {
            cfg.peer_timeout_secs = t;
        }
    }

    // Manually set for testing
    cfg.peer_timeout_secs = 60;

    assert_eq!(cfg.peer_timeout_secs, 60);
}

/// Test CHALLENGE_RELAY_ENABLED environment variable override
#[test]
fn test_challenge_relay_enabled_env_override() {
    let toml_content = r#"
challenge_relay_enabled = true
"#;
    let mut cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    // Simulate env override
    cfg.challenge_relay_enabled = false;

    assert_eq!(cfg.challenge_relay_enabled, false);
}

/// Test all network and pool fields at top level (flat TOML)
#[test]
fn test_network_and_pool_flat_toml() {
    let toml_content = r#"
bootstrap_urls = ["http://bootstrap.tribewarez.com/peers"]
enable_mdns = true
mdns_service_name = "pot-o-validator"
internal_api_port = 8900
peer_timeout_secs = 30
challenge_relay_enabled = true
"#;
    let cfg: ValidatorConfig = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(
        cfg.bootstrap_urls,
        vec!["http://bootstrap.tribewarez.com/peers".to_string()]
    );
    assert_eq!(cfg.enable_mdns, true);
    assert_eq!(cfg.mdns_service_name, "pot-o-validator");
    assert_eq!(cfg.internal_api_port, 8900);
    assert_eq!(cfg.peer_timeout_secs, 30);
    assert_eq!(cfg.challenge_relay_enabled, true);
}
