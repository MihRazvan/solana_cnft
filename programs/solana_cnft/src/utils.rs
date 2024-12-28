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

/// Transfers a compressed NFT by calling Bubblegum's transfer instruction
pub fn transfer_compressed_nft<'info>(
    bubblegum_program: AccountInfo<'info>,
    tree_authority: AccountInfo<'info>,
    leaf_owner: AccountInfo<'info>,
    new_leaf_owner: Pubkey,
    merkle_tree: AccountInfo<'info>,
    log_wrapper: AccountInfo<'info>,
    compression_program: AccountInfo<'info>,
    root: [u8; 32],
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
    nonce: u64,
    index: u32,
    proof_accounts: &[AccountInfo<'info>],
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

    let mut data = Vec::with_capacity(32 * 3 + 8 + 4);
    data.extend_from_slice(&root);
    data.extend_from_slice(&data_hash);
    data.extend_from_slice(&creator_hash);
    data.extend_from_slice(&nonce.to_le_bytes());
    data.extend_from_slice(&index.to_le_bytes());

    let ix = Instruction {
        program_id: BUBBLEGUM_ID,
        accounts,
        data,
    };

    invoke(
        &ix,
        &[
            tree_authority,
            leaf_owner,
            merkle_tree,
            log_wrapper,
            compression_program,
        ],
    ).map_err(Into::into)
}

/// Get asset ID for a cNFT
pub fn get_asset_id(merkle_tree: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"asset",
            merkle_tree.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &BUBBLEGUM_ID,
    ).0
}

/// Get vault PDA address 
pub fn get_vault_address(merkle_tree: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            crate::solana_cnft::VAULT_PREFIX,
            merkle_tree.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::ID,
    ).0
}

/// Get fraction mint authority PDA 
pub fn get_fraction_authority() -> Pubkey {
    Pubkey::find_program_address(
        &[crate::solana_cnft::AUTHORITY_PREFIX],
        &crate::ID,
    ).0
}

/// Validate metadata hashes match
pub fn validate_metadata(
    metadata: &MetadataArgs,
    data_hash: [u8; 32],
    creator_hash: [u8; 32],
) -> Result<()> {
    let computed_data_hash = hash_metadata(metadata);
    require!(
        computed_data_hash == data_hash,
        ProgramError::DataHashMismatch
    );

    if !metadata.creators.is_empty() {
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