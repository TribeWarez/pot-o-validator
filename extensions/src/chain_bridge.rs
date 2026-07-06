use async_trait::async_trait;
use pot_o_core::TribeResult;
use pot_o_mining::ProofPayload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSignature(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerAccount {
    pub pubkey: String,
    pub total_proofs: u64,
    pub total_rewards: u64,
    pub pending_rewards: u64,
    pub reputation_score: f64,
    pub last_proof_slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    SOL,
    PTtC,
    NMTC,
}

#[async_trait]
pub trait ChainBridge: Send + Sync {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature>;
    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>>;
    async fn register_miner(&self, miner_pubkey: &str) -> TribeResult<TxSignature>;
    async fn get_current_difficulty(&self) -> TribeResult<u64>;
    async fn request_swap(
        &self,
        from_token: Token,
        to_token: Token,
        amount: u64,
    ) -> TribeResult<TxSignature>;
}

#[derive(Default)]
pub struct TribechainBridge;

impl TribechainBridge {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ChainBridge for TribechainBridge {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature> {
        tracing::info!(
            challenge = %proof.proof.challenge_id,
            miner = %proof.proof.miner_pubkey,
            "TribechainBridge: submit_proof (no-op, always succeeds)"
        );
        let sig = TxSignature(format!(
            "tribechain_{}",
            &proof.proof.computation_hash[..16]
        ));
        Ok(sig)
    }

    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        tracing::debug!(
            pubkey,
            "TribechainBridge: query_miner (no-op, returning None)"
        );
        Ok(None)
    }

    async fn register_miner(&self, miner_pubkey: &str) -> TribeResult<TxSignature> {
        tracing::info!(
            miner = %miner_pubkey,
            "TribechainBridge: register_miner (no-op, always succeeds)"
        );
        Ok(TxSignature(format!("registered_{}", miner_pubkey)))
    }

    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        tracing::debug!("TribechainBridge: get_current_difficulty (returning default 2)");
        Ok(2)
    }

    async fn request_swap(
        &self,
        _from_token: Token,
        _to_token: Token,
        _amount: u64,
    ) -> TribeResult<TxSignature> {
        tracing::info!("TribechainBridge: request_swap (no-op, returning stub)");
        Ok(TxSignature("tribechain_swap_stub".to_string()))
    }
}
