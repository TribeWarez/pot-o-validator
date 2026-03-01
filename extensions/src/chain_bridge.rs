use async_trait::async_trait;
use pot_o_core::TribeResult;
use pot_o_mining::ProofPayload;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// How proofs and rewards interact with on-chain programs.
#[async_trait]
pub trait ChainBridge: Send + Sync {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature>;
    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>>;
    async fn get_current_difficulty(&self) -> TribeResult<u64>;
    async fn request_swap(
        &self,
        from_token: Token,
        to_token: Token,
        amount: u64,
    ) -> TribeResult<TxSignature>;
}

// ---------------------------------------------------------------------------
// SolanaBridge (implemented now)
// ---------------------------------------------------------------------------

pub struct SolanaBridge {
    pub rpc_url: String,
    pub program_id: String,
}

impl SolanaBridge {
    pub fn new(rpc_url: String, program_id: String) -> Self {
        Self {
            rpc_url,
            program_id,
        }
    }
}

#[async_trait]
impl ChainBridge for SolanaBridge {
    async fn submit_proof(&self, proof: &ProofPayload) -> TribeResult<TxSignature> {
        // In production this would construct an Anchor IX, sign, and send via RPC.
        // For the initial implementation we log and return a placeholder.
        tracing::info!(
            challenge = %proof.proof.challenge_id,
            miner = %proof.proof.miner_pubkey,
            "Submitting proof to Solana program {}",
            self.program_id,
        );
        Ok(TxSignature(format!(
            "sim_tx_{}",
            &proof.proof.computation_hash[..16]
        )))
    }

    async fn query_miner(&self, pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        tracing::debug!(pubkey, "Querying miner account on-chain");
        // Placeholder until Anchor program is deployed
        Ok(None)
    }

    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        // Default until on-chain config is live
        Ok(2)
    }

    async fn request_swap(
        &self,
        from_token: Token,
        to_token: Token,
        amount: u64,
    ) -> TribeResult<TxSignature> {
        tracing::info!(?from_token, ?to_token, amount, "Swap request (CPI to tribewarez-swap)");
        Ok(TxSignature("sim_swap_placeholder".into()))
    }
}

// ---------------------------------------------------------------------------
// EvmBridge (stubbed)
// ---------------------------------------------------------------------------

pub struct EvmBridge {
    pub rpc_url: String,
    pub contract_address: String,
}

#[async_trait]
impl ChainBridge for EvmBridge {
    async fn submit_proof(&self, _proof: &ProofPayload) -> TribeResult<TxSignature> {
        todo!("EVM proof submission not yet implemented")
    }
    async fn query_miner(&self, _pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        todo!("EVM miner query not yet implemented")
    }
    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        todo!("EVM difficulty query not yet implemented")
    }
    async fn request_swap(&self, _from: Token, _to: Token, _amount: u64) -> TribeResult<TxSignature> {
        todo!("EVM swap not yet implemented")
    }
}

// ---------------------------------------------------------------------------
// CrossChainBridge (stubbed)
// ---------------------------------------------------------------------------

pub struct CrossChainBridge {
    pub solana_rpc_url: String,
    pub evm_rpc_url: String,
}

#[async_trait]
impl ChainBridge for CrossChainBridge {
    async fn submit_proof(&self, _proof: &ProofPayload) -> TribeResult<TxSignature> {
        todo!("Cross-chain proof submission not yet implemented")
    }
    async fn query_miner(&self, _pubkey: &str) -> TribeResult<Option<MinerAccount>> {
        todo!("Cross-chain miner query not yet implemented")
    }
    async fn get_current_difficulty(&self) -> TribeResult<u64> {
        todo!("Cross-chain difficulty query not yet implemented")
    }
    async fn request_swap(&self, _from: Token, _to: Token, _amount: u64) -> TribeResult<TxSignature> {
        todo!("Cross-chain atomic swap not yet implemented")
    }
}
