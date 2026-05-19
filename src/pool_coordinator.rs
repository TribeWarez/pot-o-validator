//! Pool Coordinator: Manages batch proof submission and coordination with primary validator.
//!
//! The coordinator accumulates proofs from miners, batches them deterministically,
//! and submits to either the local on-chain bridge (if primary) or relays to the
//! primary validator for submission. Supports Solo, Proportional, and PPLNS strategies.

use pot_o_core::{TribeError, TribeResult};
use pot_o_extensions::pool_strategy::ProofRecord;
use std::time::{SystemTime, Duration};

// ---------------------------------------------------------------------------
// PoolCoordinator Struct
// ---------------------------------------------------------------------------

/// Manages proof batching and submission coordination for pool mining.
///
/// Accumulates proofs locally, deterministically orders them, and submits
/// batches to the primary validator (or locally if this is the primary).
#[derive(Debug, Clone)]
pub struct PoolCoordinator {
    /// This validator's unique ID
    node_id: String,
    /// Pool strategy: "solo", "proportional", or "pplns"
    pool_strategy: String,
    /// URL of primary validator for delegation (None = this is primary)
    primary_validator_url: Option<String>,
    /// Minimum proofs required to trigger batch submission
    min_batch_size: usize,
    /// Maximum age in seconds for batch before forced submission
    max_batch_age_secs: u64,
    /// Accumulated proofs waiting for batch submission
    pending_proofs: Vec<ProofRecord>,
    /// Timestamp when current batch started (None = no batch accumulating)
    batch_start_time: Option<SystemTime>,
}

impl PoolCoordinator {
    /// Create a new Pool Coordinator with given configuration.
    ///
    /// # Arguments
    /// * `node_id` - Unique identifier for this validator
    /// * `pool_strategy` - Pool mode: "solo", "proportional", or "pplns"
    /// * `primary_validator_url` - URL of primary validator (None = this is primary)
    /// * `min_batch_size` - Minimum proofs before batch submission
    /// * `max_batch_age_secs` - Maximum age before forced batch submission
    ///
    /// # Errors
    /// Returns TribeResult::Err if configuration is invalid.
    pub fn new(
        node_id: String,
        pool_strategy: String,
        primary_validator_url: Option<String>,
        min_batch_size: usize,
        max_batch_age_secs: u64,
    ) -> TribeResult<Self> {
        // Validate pool strategy
        match pool_strategy.as_str() {
            "solo" | "proportional" | "pplns" => {}
            _ => {
                return Err(TribeError::InvalidOperation(format!(
                    "invalid pool strategy: {}. must be 'solo', 'proportional', or 'pplns'",
                    pool_strategy
                )));
            }
        }

        // Validate batch parameters
        if min_batch_size == 0 {
            return Err(TribeError::InvalidOperation(
                "min_batch_size must be > 0".to_string(),
            ));
        }
        if max_batch_age_secs == 0 {
            return Err(TribeError::InvalidOperation(
                "max_batch_age_secs must be > 0".to_string(),
            ));
        }

        Ok(Self {
            node_id,
            pool_strategy,
            primary_validator_url,
            min_batch_size,
            max_batch_age_secs,
            pending_proofs: Vec::new(),
            batch_start_time: None,
        })
    }

    /// Add a proof to the current batch.
    ///
    /// Initializes batch timing on first proof added. Does not check if batch
    /// is ready for submission - caller should check `batch_ready()` after adding.
    ///
    /// # Arguments
    /// * `proof` - Proof record to add to pending batch
    ///
    /// # Returns
    /// Ok if proof added successfully
    pub fn add_proof(&mut self, proof: ProofRecord) -> TribeResult<()> {
        // Initialize batch timing if first proof
        if self.pending_proofs.is_empty() && self.batch_start_time.is_none() {
            self.batch_start_time = Some(SystemTime::now());
        }

        self.pending_proofs.push(proof);
        Ok(())
    }

    /// Get list of pending proofs waiting for submission.
    ///
    /// # Returns
    /// Vec of ProofRecord currently accumulated in batch
    pub fn get_pending_proofs(&self) -> Vec<ProofRecord> {
        self.pending_proofs.clone()
    }

    /// Check if current batch is ready for submission.
    ///
    /// Batch is ready when:
    /// - Size >= min_batch_size, OR
    /// - Time since batch start >= max_batch_age_secs
    ///
    /// # Returns
    /// true if batch should be submitted now
    pub fn batch_ready(&self) -> bool {
        // Check size
        if self.pending_proofs.len() >= self.min_batch_size {
            return true;
        }

        // Check age
        if let Some(start_time) = self.batch_start_time {
            if let Ok(elapsed) = start_time.elapsed() {
                if elapsed >= Duration::from_secs(self.max_batch_age_secs) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if this validator is the primary (receives proofs from others).
    ///
    /// Primary detection heuristic:
    /// - primary_validator_url is None, OR
    /// - primary_validator_url contains "localhost" or "127.0.0.1"
    ///
    /// # Returns
    /// true if this is the primary validator
    pub fn is_primary_validator(&self) -> bool {
        self.primary_validator_url.is_none()
            || self
                .primary_validator_url
                .as_ref()
                .map_or(false, |url| {
                    url.contains("localhost") || url.contains("127.0.0.1")
                })
    }

    /// Submit a batch of proofs for on-chain processing.
    ///
    /// If this is the primary validator, submits locally (caller should
    /// handle on-chain submission). Otherwise, relays to primary validator URL.
    ///
    /// # Arguments
    /// * `batch` - Proofs to submit (will be sorted deterministically)
    ///
    /// # Returns
    /// Result containing submission ID/hash or error
    pub async fn submit_batch_to_primary(
        &mut self,
        mut batch: Vec<ProofRecord>,
    ) -> TribeResult<String> {
        // Sort deterministically for consistent ordering
        self.sort_proofs_deterministically(&mut batch);

        // If primary, submit locally
        if self.is_primary_validator() {
            return self.submit_batch_local(&batch).await;
        }

        // Otherwise relay to primary validator
        if let Some(ref primary_url) = self.primary_validator_url {
            return self
                .submit_batch_to_url(primary_url, &batch)
                .await;
        }

        Err(TribeError::NetworkError(
            "no primary validator URL configured".to_string(),
        ))
    }

    /// Clear all pending proofs from the batch.
    ///
    /// Called after successful batch submission to reset for next batch.
    pub fn clear_batch(&mut self) {
        self.pending_proofs.clear();
        self.batch_start_time = None;
    }

    // -----------------------------------------------------------------------
    // Private Helpers
    // -----------------------------------------------------------------------

    /// Sort proofs deterministically by (miner_pubkey, timestamp).
    ///
    /// Ensures all validators produce same batch order for consistency.
    fn sort_proofs_deterministically(&self, proofs: &mut Vec<ProofRecord>) {
        proofs.sort_by(|a, b| {
            match a.miner_pubkey.cmp(&b.miner_pubkey) {
                std::cmp::Ordering::Equal => a.timestamp.cmp(&b.timestamp),
                other => other,
            }
        });
    }

    /// Submit batch locally (primary validator path).
    async fn submit_batch_local(&self, _batch: &[ProofRecord]) -> TribeResult<String> {
        // Generate submission ID (would be replaced with actual on-chain tx)
        let submission_id = format!(
            "local-batch-{}-{}",
            self.node_id,
            chrono::Utc::now().timestamp_millis()
        );

        // TODO: Call chain bridge to submit batch on-chain
        // For now, just return submission ID
        Ok(submission_id)
    }

    /// Submit batch to primary validator URL via HTTP.
    async fn submit_batch_to_url(
        &self,
        primary_url: &str,
        batch: &[ProofRecord],
    ) -> TribeResult<String> {
        let client = reqwest::Client::new();

        // Build submission payload
        let payload = serde_json::json!({
            "node_id": self.node_id,
            "pool_strategy": self.pool_strategy,
            "batch": batch,
        });

        // Submit to primary validator
        let url = format!("{}/api/pool/submit-batch", primary_url.trim_end_matches('/'));

        let response = client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                TribeError::NetworkError(format!("failed to submit batch to primary: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(TribeError::NetworkError(format!(
                "batch submission failed: {}",
                response.status()
            )));
        }

        let resp_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| {
                TribeError::NetworkError(format!("failed to parse response: {}", e))
            })?;

        resp_body
            .get("submission_id")
            .and_then(|id| id.as_str())
            .map(|id| id.to_string())
            .ok_or_else(|| {
                TribeError::NetworkError("no submission_id in response".to_string())
            })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_proof(miner_pubkey: &str, timestamp: chrono::DateTime<Utc>) -> ProofRecord {
        ProofRecord {
            miner_pubkey: miner_pubkey.to_string(),
            challenge_id: "challenge1".to_string(),
            reward: 1000,
            timestamp,
        }
    }

    fn create_coordinator(
        pool_strategy: &str,
        primary_url: Option<String>,
    ) -> TribeResult<PoolCoordinator> {
        PoolCoordinator::new(
            "validator-1".to_string(),
            pool_strategy.to_string(),
            primary_url,
            2,    // min_batch_size = 2
            3600, // max_batch_age_secs = 1 hour
        )
    }

    // =========================================================================
    // Basic Creation Tests
    // =========================================================================

    #[test]
    fn test_coordinator_creation_solo() {
        let coordinator = create_coordinator("solo", None);
        assert!(coordinator.is_ok());
        let coord = coordinator.unwrap();
        assert_eq!(coord.pool_strategy, "solo");
        assert!(coord.is_primary_validator());
    }

    #[test]
    fn test_coordinator_creation_proportional() {
        let coordinator =
            create_coordinator("proportional", Some("http://primary:8900".to_string()));
        assert!(coordinator.is_ok());
        let coord = coordinator.unwrap();
        assert_eq!(coord.pool_strategy, "proportional");
        assert!(!coord.is_primary_validator());
    }

    #[test]
    fn test_coordinator_creation_pplns() {
        let coordinator = create_coordinator("pplns", None);
        assert!(coordinator.is_ok());
        let coord = coordinator.unwrap();
        assert_eq!(coord.pool_strategy, "pplns");
    }

    #[test]
    fn test_coordinator_invalid_strategy() {
        let result = PoolCoordinator::new(
            "validator-1".to_string(),
            "invalid-strategy".to_string(),
            None,
            2,
            3600,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid pool strategy"));
    }

    #[test]
    fn test_coordinator_invalid_batch_size() {
        let result = PoolCoordinator::new(
            "validator-1".to_string(),
            "solo".to_string(),
            None,
            0, // invalid
            3600,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_coordinator_invalid_batch_age() {
        let result = PoolCoordinator::new(
            "validator-1".to_string(),
            "solo".to_string(),
            None,
            2,
            0, // invalid
        );
        assert!(result.is_err());
    }

    // =========================================================================
    // Proof Addition Tests
    // =========================================================================

    #[test]
    fn test_add_single_proof() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let proof = create_proof("miner1", Utc::now());

        let result = coordinator.add_proof(proof.clone());
        assert!(result.is_ok());

        let pending = coordinator.get_pending_proofs();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].miner_pubkey, "miner1");
    }

    #[test]
    fn test_add_multiple_proofs() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        for i in 0..5 {
            let proof = create_proof(&format!("miner{}", i), now);
            let result = coordinator.add_proof(proof);
            assert!(result.is_ok());
        }

        let pending = coordinator.get_pending_proofs();
        assert_eq!(pending.len(), 5);
    }

    #[test]
    fn test_get_pending_proofs_empty() {
        let coordinator = create_coordinator("solo", None).unwrap();
        let pending = coordinator.get_pending_proofs();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_get_pending_proofs_returns_clone() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let proof = create_proof("miner1", Utc::now());
        coordinator.add_proof(proof).unwrap();

        let pending1 = coordinator.get_pending_proofs();
        let pending2 = coordinator.get_pending_proofs();

        // Should return independent clones
        assert_eq!(pending1.len(), pending2.len());
        assert_eq!(pending1[0].miner_pubkey, pending2[0].miner_pubkey);
    }

    // =========================================================================
    // Batch Readiness Tests
    // =========================================================================

    #[test]
    fn test_batch_ready_by_size() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        // Add 1 proof - not ready
        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        assert!(!coordinator.batch_ready());

        // Add 2nd proof - should be ready (min_batch_size=2)
        coordinator.add_proof(create_proof("miner2", now)).unwrap();
        assert!(coordinator.batch_ready());
    }

    #[test]
    fn test_batch_not_ready_insufficient_size() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        // Add 1 proof with fresh start time
        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        assert!(!coordinator.batch_ready());
    }

    #[test]
    fn test_batch_ready_empty() {
        let coordinator = create_coordinator("solo", None).unwrap();
        assert!(!coordinator.batch_ready());
    }

    #[test]
    fn test_batch_start_time_initialized_on_first_proof() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        assert!(coordinator.batch_start_time.is_none());

        coordinator.add_proof(create_proof("miner1", Utc::now())).unwrap();
        assert!(coordinator.batch_start_time.is_some());
    }

    #[test]
    fn test_batch_start_time_not_reset_on_second_proof() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let first_time = Utc::now();

        coordinator.add_proof(create_proof("miner1", first_time)).unwrap();
        let first_batch_time = coordinator.batch_start_time;

        coordinator.add_proof(create_proof("miner2", Utc::now())).unwrap();
        let second_batch_time = coordinator.batch_start_time;

        // Should be same start time
        assert_eq!(first_batch_time, second_batch_time);
    }

    // =========================================================================
    // Primary Validator Detection Tests
    // =========================================================================

    #[test]
    fn test_is_primary_validator_none_url() {
        let coordinator = create_coordinator("solo", None).unwrap();
        assert!(coordinator.is_primary_validator());
    }

    #[test]
    fn test_is_primary_validator_localhost() {
        let coordinator =
            create_coordinator("solo", Some("http://localhost:8900".to_string())).unwrap();
        assert!(coordinator.is_primary_validator());
    }

    #[test]
    fn test_is_primary_validator_127_0_0_1() {
        let coordinator =
            create_coordinator("solo", Some("http://127.0.0.1:8900".to_string())).unwrap();
        assert!(coordinator.is_primary_validator());
    }

    #[test]
    fn test_is_primary_validator_remote_url() {
        let coordinator = create_coordinator("solo", Some("http://primary.cluster:8900".to_string()))
            .unwrap();
        assert!(!coordinator.is_primary_validator());
    }

    #[test]
    fn test_is_primary_validator_remote_ip() {
        let coordinator =
            create_coordinator("solo", Some("http://192.168.1.100:8900".to_string())).unwrap();
        assert!(!coordinator.is_primary_validator());
    }

    // =========================================================================
    // Batch Clearing Tests
    // =========================================================================

    #[test]
    fn test_clear_batch() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        coordinator.add_proof(create_proof("miner2", now)).unwrap();

        assert_eq!(coordinator.get_pending_proofs().len(), 2);
        assert!(coordinator.batch_start_time.is_some());

        coordinator.clear_batch();

        assert_eq!(coordinator.get_pending_proofs().len(), 0);
        assert!(coordinator.batch_start_time.is_none());
    }

    #[test]
    fn test_clear_batch_twice() {
        let mut coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        coordinator.clear_batch();
        coordinator.clear_batch(); // Should not panic

        assert_eq!(coordinator.get_pending_proofs().len(), 0);
    }

    // =========================================================================
    // Deterministic Sorting Tests
    // =========================================================================

    #[test]
    fn test_deterministic_sorting_by_pubkey() {
        let coordinator = create_coordinator("solo", None).unwrap();
        let now = Utc::now();

        let mut batch = vec![
            create_proof("zebra_miner", now),
            create_proof("alpha_miner", now),
            create_proof("beta_miner", now),
        ];

        coordinator.sort_proofs_deterministically(&mut batch);

        assert_eq!(batch[0].miner_pubkey, "alpha_miner");
        assert_eq!(batch[1].miner_pubkey, "beta_miner");
        assert_eq!(batch[2].miner_pubkey, "zebra_miner");
    }

    #[test]
    fn test_deterministic_sorting_by_timestamp_same_pubkey() {
        let coordinator = create_coordinator("solo", None).unwrap();
        let base_time = Utc::now();

        let mut batch = vec![
            create_proof("miner1", base_time + chrono::Duration::seconds(3)),
            create_proof("miner1", base_time + chrono::Duration::seconds(1)),
            create_proof("miner1", base_time + chrono::Duration::seconds(2)),
        ];

        coordinator.sort_proofs_deterministically(&mut batch);

        assert!(batch[0].timestamp <= batch[1].timestamp);
        assert!(batch[1].timestamp <= batch[2].timestamp);
    }

    #[test]
    fn test_deterministic_sorting_combined() {
        let coordinator = create_coordinator("solo", None).unwrap();
        let base_time = Utc::now();

        let mut batch = vec![
            create_proof("miner2", base_time + chrono::Duration::seconds(1)),
            create_proof("miner1", base_time + chrono::Duration::seconds(3)),
            create_proof("miner1", base_time + chrono::Duration::seconds(1)),
            create_proof("miner2", base_time + chrono::Duration::seconds(2)),
        ];

        coordinator.sort_proofs_deterministically(&mut batch);

        // All miner1 should come before miner2
        assert_eq!(batch[0].miner_pubkey, "miner1");
        assert_eq!(batch[1].miner_pubkey, "miner1");
        assert_eq!(batch[2].miner_pubkey, "miner2");
        assert_eq!(batch[3].miner_pubkey, "miner2");

        // Within each miner, should be sorted by timestamp
        assert!(batch[0].timestamp <= batch[1].timestamp);
        assert!(batch[2].timestamp <= batch[3].timestamp);
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    #[test]
    fn test_full_workflow_primary() {
        let mut coordinator = create_coordinator("proportional", None).unwrap();
        let now = Utc::now();

        // Add proofs
        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        assert!(!coordinator.batch_ready());

        coordinator.add_proof(create_proof("miner2", now)).unwrap();
        assert!(coordinator.batch_ready());

        // Get batch and verify it's cloned
        let batch = coordinator.get_pending_proofs();
        assert_eq!(batch.len(), 2);

        // Clear batch
        coordinator.clear_batch();
        assert_eq!(coordinator.get_pending_proofs().len(), 0);
        assert!(!coordinator.batch_ready());
    }

    #[test]
    fn test_full_workflow_secondary() {
        let mut coordinator = create_coordinator(
            "pplns",
            Some("http://primary.cluster:8900".to_string()),
        )
        .unwrap();
        let now = Utc::now();

        assert!(!coordinator.is_primary_validator());

        coordinator.add_proof(create_proof("miner1", now)).unwrap();
        coordinator.add_proof(create_proof("miner2", now)).unwrap();

        assert!(coordinator.batch_ready());

        let batch = coordinator.get_pending_proofs();
        assert_eq!(batch.len(), 2);

        coordinator.clear_batch();
        assert_eq!(coordinator.get_pending_proofs().len(), 0);
    }
}
