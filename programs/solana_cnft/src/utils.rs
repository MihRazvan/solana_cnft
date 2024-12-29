use anchor_lang::prelude::*;
use solana_program::{
    keccak,
    system_program,
    program::invoke,
    instruction::{AccountMeta, Instruction},
};
use mpl_bubblegum::{
    ID as BUBBLEGUM_ID,
    hash::{hash_metadata, hash_creators},
    types::MetadataArgs,
};

use crate::error::ErrorCode as ProgramError;

pub fn transfer_compressed_nft<'a>(
    bubblegum_program: &AccountInfo<'a>,
    tree_authority: &AccountInfo<'a>,
    leaf_owner: &AccountInfo<'a>,
    new_leaf_owner: Pubkey,
    merkle_tree: &AccountInfo<'a>,
    log_wrapper: &AccountInfo<'a>,
    compression_program: &AccountInfo<'a>,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
    proof_accounts: &[AccountInfo<'a>],
) -> Result<()> {
    let mut accounts = vec![
        AccountMeta::new_readonly(tree_authority.key(), false),
        AccountMeta::new_readonly(leaf_owner.key(), true),
        AccountMeta::new_readonly(new_leaf_owner, false),
        AccountMeta::new(merkle_tree.key(), false),
        AccountMeta::new_readonly(log_wrapper.key(), false),
        AccountMeta::new_readonly(compression_program.key(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    accounts.extend(proof_accounts.iter().map(|a| AccountMeta::new_readonly(a.key(), false)));

    let ix = Instruction {
        program_id: BUBBLEGUM_ID,
        accounts,
        data: create_transfer_data(&root, &data_hash, &creator_hash, nonce, index).to_vec(),
    };    

    invoke(
        &ix,
        &[
            tree_authority.clone(),
            leaf_owner.clone(),
            merkle_tree.clone(),
            log_wrapper.clone(),
            compression_program.clone(),
        ],
    ).map_err(Into::into)
}

/// Validate metadata hashes match
pub fn validate_metadata(
    metadata: &MetadataArgs,
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
) -> Result<()> {
    // hash_metadata returns a Result, so we need to handle it
    let computed_data_hash = hash_metadata(metadata)?;
    require!(
        computed_data_hash == data_hash,
        ProgramError::DataHashMismatch
    );

    if !metadata.creators.is_empty() {
        // hash_creators returns [u8; 32] directly
        let computed_creator_hash = hash_creators(&metadata.creators);
        require!(
            computed_creator_hash == creator_hash,
            ProgramError::DataHashMismatch
        );
    }

    Ok(())
}

/// Calculate unique fraction amount based on asset hashes
pub fn calculate_fraction_amount(data_hash: &[u8; 32], creator_hash: &[u8; 32]) -> u64 {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(data_hash);
    combined.extend_from_slice(creator_hash);
    
    let hash = keccak::hashv(&[&combined]);
    let first_8_bytes = &hash.to_bytes()[0..8];
    let base_amount = u64::from_le_bytes(first_8_bytes.try_into().unwrap());
    
    // Range: 100-10000
    (base_amount % 9900) + 100
}

const TRANSFER_DATA_SIZE: usize = 32 * 3 + 8 + 4;

fn create_transfer_data(
    root: &[u8; 32],
    data_hash: &[u8; 32],
    creator_hash: &[u8; 32],
    nonce: u64,
    index: u32,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(TRANSFER_DATA_SIZE);
    data.extend_from_slice(root);
    data.extend_from_slice(data_hash);
    data.extend_from_slice(creator_hash);
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(&index.to_le_bytes());
    data
}