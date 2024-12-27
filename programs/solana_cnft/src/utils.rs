use anchor_lang::prelude::*;
use solana_program::{keccak, program::invoke};
use mpl_bubblegum::{
    programs::MPL_BUBBLEGUM_ID,
    accounts::TreeConfig,
    types::MetadataArgs,
    hash::{hash_metadata, hash_creators},
};
use spl_account_compression::{
    programs::SPL_ACCOUNT_COMPRESSION_ID,
    Node,
};
use solana_program::keccak;

/// Transfers a compressed NFT by calling Bubblegum's transfer instruction
pub fn transfer_compressed_nft<'info>(
    bubblegum_program: AccountInfo<'info>,
    tree_authority: AccountInfo<'info>,
    from_owner: AccountInfo<'info>,
    to_owner: Pubkey,
    merkle_tree: AccountInfo<'info>,
    log_wrapper: AccountInfo<'info>,
    compression_program: AccountInfo<'info>,
    root: [u8; 32],
    leaf_node: Node,
    new_leaf_node: Node,
    index: u32,
    proof_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    // Construct accounts for transfer
    let mut accounts = Vec::with_capacity(8 + proof_accounts.len());
    accounts.extend([
        AccountMeta::new_readonly(tree_authority.key(), false),
        AccountMeta::new_readonly(from_owner.key(), true), 
        AccountMeta::new_readonly(from_owner.key(), false), // delegate
        AccountMeta::new_readonly(to_owner, false),
        AccountMeta::new(merkle_tree.key(), false),
        AccountMeta::new_readonly(log_wrapper.key(), false),
        AccountMeta::new_readonly(compression_program.key(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ]);

    // Add proof accounts
    for proof_account in proof_accounts.iter() {
        accounts.push(AccountMeta::new_readonly(proof_account.key(), false));
    }

    // Construct transfer instruction data
    let mut data = Vec::with_capacity(1 + 32 + 32 + 32 + 8 + 4);
    data.extend([163, 52, 200, 231, 140, 3, 69, 186]); // Transfer discriminator
    data.extend(root);
    data.extend(leaf_node);
    data.extend(new_leaf_node);
    data.extend((index as u64).to_le_bytes());

    // Build transfer instruction
    let transfer_ix = solana_program::instruction::Instruction {
        program_id: mpl_bubblegum::ID(),
        accounts,
        data,
    };

    // Collect account infos
    let mut account_infos = Vec::with_capacity(8 + proof_accounts.len());
    account_infos.extend([
        tree_authority.clone(),
        from_owner.clone(),
        from_owner.clone(), // delegate
        merkle_tree.clone(),
        log_wrapper.clone(),
        compression_program.clone(),
        solana_program::system_program::id().clone(),
    ]);
    account_infos.extend(proof_accounts.iter().cloned());

    // Execute transfer
    invoke(
        &transfer_ix,
        &account_infos
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
        &mpl_bubblegum::ID(),
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
    let computed_data_hash = mpl_bubblegum::hash::hash_metadata(metadata)?;
    require!(
        computed_data_hash == data_hash,
        crate::error::ErrorCode::DataHashMismatch
    );

    // Validate creator hash if creators exist
    if !metadata.creators.is_empty() {
        let computed_creator_hash = mpl_bubblegum::hash::hash_creators(&metadata.creators)?;
        require!(
            computed_creator_hash == creator_hash,
            crate::error::ErrorCode::DataHashMismatch  
        );
    }

    Ok(())
}

pub fn transfer_compressed_nft<'info>(
    bubblegum_program: AccountInfo<'info>,
    tree_authority: AccountInfo<'info>,
    from_owner: AccountInfo<'info>,
    to_owner: Pubkey,
    merkle_tree: AccountInfo<'info>,
    log_wrapper: AccountInfo<'info>,
    compression_program: AccountInfo<'info>,
    root: [u8; 32],
    leaf_node: Node,
    new_leaf_node: Node,
    index: u32,
    proof_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    let mut accounts = Vec::with_capacity(8);
    accounts.extend([
        AccountMeta::new_readonly(tree_authority.key(), false),
        AccountMeta::new_readonly(from_owner.key(), true),
        AccountMeta::new_readonly(to_owner, false),
        AccountMeta::new(merkle_tree.key(), false),
        AccountMeta::new_readonly(log_wrapper.key(), false),
        AccountMeta::new_readonly(compression_program.key(), false),
        AccountMeta::new_readonly(System::id(), false),
    ]);
    accounts.extend(proof_accounts.iter().map(|a| AccountMeta::new_readonly(a.key(), false)));

    invoke(
        &Instruction {
            program_id: MPL_BUBBLEGUM_ID,
            accounts,
            data: [root, leaf_node, new_leaf_node, index.to_le_bytes()].concat(),
        },
        &[tree_authority, from_owner, merkle_tree, log_wrapper, compression_program]
    ).map_err(Into::into)
}