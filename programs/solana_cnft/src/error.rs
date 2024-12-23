use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Not the NFT owner")]
    InvalidOwner,
    #[msg("Invalid metadata provided")]
    DataHashMismatch,
    #[msg("Invalid leaf authority")]
    LeafAuthorityMustSign,
    #[msg("Invalid proof length")]
    InvalidProofLength,
    #[msg("Merkle proof verification failed")]
    MerkleProofVerificationFailed,
    #[msg("NFT not in vault")]
    NFTNotInVault,
    #[msg("Invalid tree owner")]
    InvalidTreeOwner,
    #[msg("Invalid tree state")]
    InvalidTreeState,
    #[msg("Tree update failed")]
    TreeUpdateFailed,
    #[msg("Invalid proof")]
    InvalidProof,
    #[msg("Invalid canopy state")]
    InvalidCanopyState,
    #[msg("Insufficient fraction token balance")]
    InsufficientFractionBalance,
}