use anchor_lang::prelude::*;
use mpl_bubblegum::program::ID as BUBBLEGUM_ID;
use spl_account_compression::program::ID as COMPRESSION_ID;

#[account]
pub struct Vault {
    pub owner: Pubkey,                // 32
    pub merkle_tree: Pubkey,          // 32
    pub root: [u8; 32],               // 32
    pub data_hash: [u8; 32],          // 32
    pub creator_hash: [u8; 32],       // 32
    pub nonce: u64,                   // 8
    pub index: u32,                   // 4
    pub locked_at: i64,               // 8
}

impl Vault {
    pub const LEN: usize = 8 +        // discriminator
        32 +                          // owner
        32 +                          // merkle_tree
        32 +                          // root
        32 +                          // data_hash
        32 +                          // creator_hash
        8 +                           // nonce
        4 +                           // index
        8;                            // locked_at
    
    /// Creates a new vault address
    pub fn pda(merkle_tree: &Pubkey, nonce: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[
                crate::solana_cnft::VAULT_PREFIX,
                merkle_tree.as_ref(),
                &nonce.to_le_bytes(),
            ],
            &crate::ID,
        )
    }
}

/// Find PDA for tree authority
pub fn find_tree_authority(merkle_tree: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[merkle_tree.as_ref()],
        &mpl_bubblegum::ID,
    )
}

/// Find PDA for mint authority
pub fn find_authority() -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[crate::solana_cnft::AUTHORITY_PREFIX],
        &crate::ID,
    )
}

/// Validates a vault's ownership
pub fn assert_vault_owner(
    vault: &Vault,
    expected_owner: &Pubkey,
    program_id: &Pubkey,
) -> Result<()> {
    require!(
        vault.owner == *expected_owner,
        crate::error::ErrorCode::InvalidOwner
    );

    // Get vault PDA
    let (vault_pda, _) = Vault::pda(&vault.merkle_tree, vault.nonce);
    
    // Verify vault is owned by program
    require!(
        vault.owner == vault_pda,
        crate::error::ErrorCode::InvalidOwner  
    );

    Ok(())
}

/// Calculates asset ID for a cNFT
pub fn get_asset_id(merkle_tree: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"asset",
            merkle_tree.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &mpl_bubblegum::ID,
    ).0
}

pub fn validate_state(
    &self,
    merkle_tree: &AccountInfo,
    owner: &Pubkey,
) -> Result<()> {
    // Validate owner
    require!(
        self.owner == *owner,
        ErrorCode::InvalidOwner
    );

    // Validate merkle tree ownership
    require!(
        merkle_tree.owner == &spl_account_compression::id(),
        ErrorCode::InvalidTreeOwner
    );

    // Validate tree state matches
    require!(
        self.merkle_tree == merkle_tree.key(),
        ErrorCode::InvalidTreeState  
    );

    Ok(())
}