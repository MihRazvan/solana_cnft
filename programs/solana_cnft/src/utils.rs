use mpl_bubblegum::{
    hash::{hash_metadata, hash_creators},
    accounts::TreeConfig,
    programs::MPL_BUBBLEGUM_ID,
};
use spl_account_compression::{Node, programs::SPL_ACCOUNT_COMPRESSION_ID};

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

/// Verifies a merkle proof
pub fn verify_merkle_proof(
    merkle_tree: &AccountInfo,
    root: [u8; 32],
    leaf: Node, 
    index: u32,
    proof_accounts: &[AccountInfo],
) -> Result<()> {
    // Construct accounts for verification
    let mut accounts = Vec::with_capacity(1 + proof_accounts.len());
    accounts.push(AccountMeta::new_readonly(merkle_tree.key(), false));
    
    // Add proof accounts
    for proof_account in proof_accounts.iter() {
        accounts.push(AccountMeta::new_readonly(proof_account.key(), false));
    }

    // Build verify instruction
    let verify_ix = spl_account_compression::instruction::verify_leaf(
        merkle_tree.key(),
        root,
        leaf,
        index,
    );

    // Collect account infos  
    let mut account_infos = Vec::with_capacity(1 + proof_accounts.len());
    account_infos.push(merkle_tree.clone());
    account_infos.extend(proof_accounts.iter().cloned());

    // Execute verification
    invoke(
        &verify_ix,
        &account_infos
    ).map_err(|_| error!(crate::error::ErrorCode::MerkleProofVerificationFailed))
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

/// Verify merkle tree state and configuration
pub fn verify_tree_state(
    merkle_tree: &AccountInfo,
    tree_authority: &Account<TreeConfig>,
    max_depth: u32,
    max_buffer_size: u32,
) -> Result<()> {
    // Verify merkle tree ownership
    require!(
        merkle_tree.owner == &spl_account_compression::id(),
        crate::error::ErrorCode::InvalidTreeOwner
    );

    // Verify tree authority derivation
    let (expected_authority, _) = Pubkey::find_program_address(
        &[merkle_tree.key().as_ref()],
        &mpl_bubblegum::ID(),
    );
    require!(
        tree_authority.key() == expected_authority,
        crate::error::ErrorCode::InvalidTreeAuthority
    );

    // Verify tree configuration
    require!(
        tree_authority.total_mint_capacity == 1 << max_depth,
        crate::error::ErrorCode::InvalidTreeState
    );

    Ok(())
}

/// Helper to get canonical bump for PDAs
pub fn get_canonical_bump(seeds: &[&[u8]], program_id: &Pubkey) -> u8 {
    Pubkey::find_program_address(seeds, program_id).1
}

pub fn calculate_fraction_amount(data_hash: &[u8; 32], creator_hash: &[u8; 32]) -> u64 {
    // Use both hashes to generate unique but deterministic amount
    let combined = [data_hash, creator_hash].concat();
    let hash = keccak::hashv(&[&combined]);
    let first_8_bytes = &hash.to_bytes()[0..8];
    let base_amount = u64::from_le_bytes(first_8_bytes.try_into().unwrap());
    
    // Ensure amount is within reasonable range (100-10000)
    (base_amount % 9900) + 100
}