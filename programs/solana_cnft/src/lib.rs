use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke_signed, instruction::AccountMeta},
};
use anchor_spl::{
    token::{Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};

use mpl_bubblegum::{
    program::Bubblegum,
    state::{
        leaf_schema::LeafSchema,
        TreeConfig,
    },
    types::{
        Collection, Creator, MetadataArgs,
        TokenProgramVersion, TokenStandard,
    },
    hash::{hash_metadata, hash_creators},
};

use spl_account_compression::{
    program::SplAccountCompression,
    wrap_application_data_v1,
    Noop,
    Node,
};

declare_id!("91CLwQaCxutnTf8XafP3e6EmGBA3eUkMaw86Hgghax2L");

#[program]
pub mod solana_cnft {
    use super::*;

    pub const FRACTION_AMOUNT: u64 = 1_000;
    pub const FRACTION_DECIMALS: u8 = 0;
    pub const VAULT_PREFIX: &[u8] = b"vault";
    pub const AUTHORITY_PREFIX: &[u8] = b"authority";

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn lock_cnft(
        ctx: Context<LockCNFT>,
        metadata: MetadataArgs,
        root: [u8; 32],
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
        nonce: u64,
        index: u32,
    ) -> Result<()> {
        // Validate metadata
        let computed_data_hash = hash_metadata(&metadata)?;
        require!(
            computed_data_hash == data_hash,
            ErrorCode::DataHashMismatch
        );

        // Create vault account with compressed NFT data
        let vault = &mut ctx.accounts.vault;
        vault.owner = ctx.accounts.owner.key();
        vault.asset_id = get_asset_id(&ctx.accounts.merkle_tree.key(), nonce);
        vault.merkle_tree = ctx.accounts.merkle_tree.key();
        vault.root = root;
        vault.data_hash = data_hash;  
        vault.creator_hash = creator_hash;
        vault.nonce = nonce;
        vault.index = index;
        vault.locked_at = Clock::get()?.unix_timestamp;

        // Create LeafSchema for previous and new state
        let previous_leaf = LeafSchema::V1 {
            id: vault.asset_id,
            owner: ctx.accounts.owner.key(),
            delegate: ctx.accounts.owner.key(),
            nonce,
            data_hash,
            creator_hash,
        };

        // New state owned by vault PDA
        let vault_key = vault.key();
        let new_leaf = LeafSchema::V1 {
            id: vault.asset_id,
            owner: vault_key,
            delegate: vault_key,
            nonce,
            data_hash,
            creator_hash,
        };

        // Log the state change
        wrap_application_data_v1(
            new_leaf.try_to_vec()?,
            &ctx.accounts.log_wrapper.to_account_info(),
        )?;

        // Replace leaf in merkle tree
        replace_leaf(
            &ctx.accounts.merkle_tree.key(),
            *ctx.bumps.get("tree_authority").unwrap(),
            &ctx.accounts.compression_program.to_account_info(),
            &ctx.accounts.tree_authority.to_account_info(),
            &ctx.accounts.merkle_tree.to_account_info(),
            &ctx.accounts.log_wrapper.to_account_info(),
            ctx.remaining_accounts,
            root,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            index,
        )?;

        // Mint fractions to owner
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.fraction_mint.to_account_info(),
                    to: ctx.accounts.owner_fraction_account.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
                &[&[AUTHORITY_PREFIX, &[ctx.bumps.authority]]],
            ),
            FRACTION_AMOUNT,
        )?;

        msg!("cNFT locked in vault: {}", vault.key());
        Ok(())
    }

    pub fn unlock_cnft(ctx: Context<UnlockCNFT>) -> Result<()> {
        let vault = &ctx.accounts.vault;

        // Verify ownership through fractions
        require!(
            vault.owner == ctx.accounts.owner.key(),
            ErrorCode::InvalidOwner
        );

        // Recreate leaf states for unlocking 
        let vault_key = vault.key();
        let previous_leaf = LeafSchema::V1 {
            id: vault.asset_id,
            owner: vault_key,
            delegate: vault_key,
            nonce: vault.nonce,
            data_hash: vault.data_hash,
            creator_hash: vault.creator_hash,
        };

        let new_leaf = LeafSchema::V1 {
            id: vault.asset_id,
            owner: ctx.accounts.owner.key(),
            delegate: ctx.accounts.owner.key(),
            nonce: vault.nonce,
            data_hash: vault.data_hash,
            creator_hash: vault.creator_hash,
        };

        // Log state change
        wrap_application_data_v1(
            new_leaf.try_to_vec()?,
            &ctx.accounts.log_wrapper.to_account_info(),
        )?;

        // Replace leaf in merkle tree
        replace_leaf(
            &ctx.accounts.merkle_tree.key(),
            *ctx.bumps.get("tree_authority").unwrap(),
            &ctx.accounts.compression_program.to_account_info(),
            &ctx.accounts.tree_authority.to_account_info(),
            &ctx.accounts.merkle_tree.to_account_info(),
            &ctx.accounts.log_wrapper.to_account_info(),
            ctx.remaining_accounts,
            vault.root,
            previous_leaf.to_node(),
            new_leaf.to_node(),
            vault.index,
        )?;

        // Burn fractions
        anchor_spl::token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.fraction_mint.to_account_info(),
                    from: ctx.accounts.owner_fraction_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            FRACTION_AMOUNT,
        )?;

        msg!("Unlocking cNFT from vault: {}", vault.key());
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    // ... [Previous Initialize implementation remains the same]
}

#[derive(Accounts)]
pub struct LockCNFT<'info> {
    // ... [Previous LockCNFT implementation remains the same]
}

#[derive(Accounts)]
pub struct UnlockCNFT<'info> {
    // ... [Previous UnlockCNFT implementation remains the same]
}

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub asset_id: Pubkey,
    pub merkle_tree: Pubkey,
    pub root: [u8; 32],
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
    pub nonce: u64,
    pub index: u32,
    pub locked_at: i64,
}

impl Vault {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // asset_id
        32 + // merkle_tree
        32 + // root
        32 + // data_hash
        32 + // creator_hash
        8 +  // nonce
        4 +  // index
        8;   // locked_at
}

#[error_code]
pub enum ErrorCode {
    #[msg("Not the NFT owner")]
    InvalidOwner,
    #[msg("Invalid metadata provided")]
    DataHashMismatch,
    #[msg("Invalid leaf authority")]
    LeafAuthorityMustSign,
}

// Helper function to get asset ID consistently
fn get_asset_id(merkle_tree: &Pubkey, nonce: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[
            b"asset",
            merkle_tree.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &mpl_bubblegum::id(),
    ).0
}

// Helper function for replacing leaf in merkle tree
fn replace_leaf<'info>(
    seed: &Pubkey,
    bump: u8,
    compression_program: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    merkle_tree: &AccountInfo<'info>,
    log_wrapper: &AccountInfo<'info>,
    remaining_accounts: &[AccountInfo<'info>],
    root: [u8; 32],
    previous_leaf: Node,
    new_leaf: Node,
    index: u32,
) -> Result<()> {
    let seeds = &[seed.as_ref(), &[bump]];
    let authority_pda_signer = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        compression_program.clone(),
        spl_account_compression::cpi::accounts::Modify {
            authority: authority.clone(),
            merkle_tree: merkle_tree.clone(),
            noop: log_wrapper.clone(),
        },
        authority_pda_signer,
    )
    .with_remaining_accounts(remaining_accounts.to_vec());

    spl_account_compression::cpi::replace_leaf(
        cpi_ctx,
        root,
        previous_leaf,
        new_leaf,
        index
    )
}