use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};
use mpl_bubblegum::{
    state::TreeConfig,
    types::MetadataArgs,
    hash::{hash_metadata, hash_creators},
};
use spl_account_compression::{program::SplAccountCompression, wrap_application_data_v1, Noop};

use crate::{error::ErrorCode, state::Vault};

#[derive(Accounts)]
#[instruction(asset_id: Pubkey)]
pub struct LockCNFT<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = Vault::LEN,
        seeds = [
            crate::solana_cnft::VAULT_PREFIX,
            merkle_tree.key().as_ref(),
            &nonce.to_le_bytes()
        ],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        seeds = [merkle_tree.key().as_ref()],
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    /// CHECK: Validated by seeds
    pub tree_authority: Account<'info, TreeConfig>,

    /// CHECK: Validated through cpi
    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,

    #[account(mut)]
    pub fraction_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = fraction_mint,
        associated_token::authority = owner,
    )]
    pub owner_fraction_account: Account<'info, TokenAccount>,

    /// CHECK: PDA for fraction mint authority
    #[account(
        seeds = [crate::solana_cnft::AUTHORITY_PREFIX],
        bump
    )]
    pub authority: UncheckedAccount<'info>,

    pub bubblegum_program: Program<'info, mpl_bubblegum::program::Bubblegum>,
    pub log_wrapper: Program<'info, Noop>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
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
    vault.merkle_tree = ctx.accounts.merkle_tree.key();
    vault.root = root;
    vault.data_hash = data_hash;
    vault.creator_hash = creator_hash;
    vault.nonce = nonce;
    vault.index = index;
    vault.locked_at = Clock::get()?.unix_timestamp;

    // Create leaf schema for previous and new state
    let previous_leaf = LeafSchema::V1 {
        id: crate::utils::get_asset_id(&ctx.accounts.merkle_tree.key(), nonce),
        owner: ctx.accounts.owner.key(),
        delegate: ctx.accounts.owner.key(),
        nonce,
        data_hash,
        creator_hash,
    };

    let vault_key = vault.key();
    let new_leaf = LeafSchema::V1 {
        id: crate::utils::get_asset_id(&ctx.accounts.merkle_tree.key(), nonce),
        owner: vault_key,
        delegate: vault_key,
        nonce,
        data_hash,
        creator_hash,
    };

    // Log state change
    wrap_application_data_v1(
        new_leaf.try_to_vec()?,
        &ctx.accounts.log_wrapper.to_account_info(),
    )?;

    // Transfer NFT to vault via CPI to Bubblegum
    crate::utils::transfer_compressed_nft(
        ctx.accounts.bubblegum_program.to_account_info(),
        ctx.accounts.tree_authority.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        vault_key,
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.accounts.compression_program.to_account_info(),
        root,
        previous_leaf.to_node(),
        new_leaf.to_node(),
        index,
        ctx.remaining_accounts,
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
            &[&[
                crate::solana_cnft::AUTHORITY_PREFIX,
                &[*ctx.bumps.get("authority").unwrap()]
            ]],
        ),
        crate::solana_cnft::FRACTION_AMOUNT,
    )?;

    msg!("cNFT locked in vault: {}", vault.key());
    Ok(())
}