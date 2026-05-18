use serde::{Deserialize, Serialize};

pub const HASH_BYTES: usize = 32;
pub const NEIGHBOR_SLOTS: usize = 12;

pub type BlockHash = [u8; HASH_BYTES];

pub const NEIGHBOR_SLOT_EMPTY: BlockHash = [0u8; HASH_BYTES];

pub fn is_empty_neighbor_slot(h: &BlockHash) -> bool {
    *h == NEIGHBOR_SLOT_EMPTY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    StructuralInvalid,
    LatticeMismatch,
    NoMatureNeighbors,
    PowTooHigh,
    MmlExceeded,
    MissingPrevBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MmlParams {
    pub max_num: u64,
    pub max_den: u64,
}

impl Default for MmlParams {
    fn default() -> Self {
        Self {
            max_num: 95,
            max_den: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorMeta {
    pub expected_capacity: u64,
    pub actual_capacity: u64,
    pub compression_num: u64,
    pub compression_den: u64,
}

impl Default for TensorMeta {
    fn default() -> Self {
        Self {
            expected_capacity: 0,
            actual_capacity: 0,
            compression_num: 1,
            compression_den: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsensusParams {
    pub maturity_depth: u64,
    pub symmetry_num: u64,
    pub symmetry_den: u64,
    pub base_target: BlockHash,
    pub mml: MmlParams,
}

impl Default for ConsensusParams {
    fn default() -> Self {
        Self {
            maturity_depth: 10,
            symmetry_num: 115,
            symmetry_den: 100,
            base_target: [0xFFu8; HASH_BYTES],
            mml: MmlParams::default(),
        }
    }
}
