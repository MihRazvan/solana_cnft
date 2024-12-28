use anchor_lang::prelude::*;
use solana_program::{keccak, program::invoke};
use mpl_bubblegum::{
    programs::MPL_BUBBLEGUM_ID,
    accounts::TreeConfig,
    types::MetadataArgs,
    hash::{hash_metadata, hash_creators},
    instructions::TransferCpiBuilder,
};
use spl_account_compression::{
    programs::SPL_ACCOUNT_COMPRESSION_ID,
    Node,
};

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
    let transfer = TransferCpiBuilder::new(bubblegum_program.clone())
        .tree_authority(tree_authority.clone())
        .leaf_owner(leaf_owner.clone())
        .new_leaf_owner(new_leaf_owner)
        .merkle_tree(merkle_tree.clone())
        .log_wrapper(log_wrapper.clone())
        .compression_program(compression_program.clone())
        .root(root)
        .data_hash(data_hash)
        .creator_hash(creator_hash)
        .nonce(nonce)
        .index(index);

    for proof_account in proof_accounts {
        transfer.add_remaining_account(proof_account, false, false);
    }

    transfer.invoke()
}

/// Get asset ID for a cNFT
pub fn get_asset_id(merkle_tree: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"asset",
            merkle_tree.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &MPL_BUBBLEGUM_ID,
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
    // Validate data hash
    let computed_data_hash = hash_metadata(metadata)?;
    require!(
        computed_data_hash == data_hash,
        crate::error::ErrorCode::DataHashMismatch
    );

    // Validate creator hash if creators exist
    if !metadata.creators.is_empty() {
        let computed_creator_hash = hash_creators(&metadata.creators)?;
        require!(
            computed_creator_hash == creator_hash,
            crate::error::ErrorCode::DataHashMismatch  
        );
    }

    Ok(())
}

/// Calculate unique fraction amount for a cNFT based on its hashes
pub fn calculate_fraction_amount(data_hash: &[u8; 32], creator_hash: &[u8; 32]) -> u64 {
    // Combine both hashes to ensure uniqueness
    let combined = [data_hash, creator_hash].concat();
    let hash = keccak::hashv(&[&combined]);
    
    // Use first 8 bytes as a basis for amount
    let first_8_bytes = &hash.to_bytes()[0..8];
    let base_amount = u64::from_le_bytes(first_8_bytes.try_into().unwrap());
    
    // Range: 100-10000
    (base_amount % 9900) + 100
}