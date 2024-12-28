use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint, mint_to},
    associated_token::AssociatedToken,
};
use mpl_bubblegum::{
    ID as BUBBLEGUM_ID,
    hash::hash_metadata,
    types::MetadataArgs,
};
use spl_account_compression::{
    ID as COMPRESSION_ID,
    Noop,
};

use crate::{
    error::ErrorCode,
    state::Vault,
    utils::{calculate_fraction_amount, validate_metadata},
};

#[derive(Accounts)]
#[instruction(metadata: MetadataArgs, nonce: u64)]
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
        seeds::program = BUBBLEGUM_ID,
        bump,
    )]
    /// CHECK: Validated through seeds
    pub tree_authority: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated through CPI
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

    /// CHECK: Bubblegum program
    pub bubblegum_program: Program<'info, Noop>,
    pub log_wrapper: Program<'info, Noop>,
    /// CHECK: Compression program
    pub compression_program: Program<'info, Noop>,
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
    validate_metadata(&metadata, data_hash, creator_hash)?;

    // Initialize vault data
    let vault = &mut ctx.accounts.vault;
    vault.owner = ctx.accounts.owner.key();
    vault.merkle_tree = ctx.accounts.merkle_tree.key();
    vault.root = root;
    vault.data_hash = data_hash;
    vault.creator_hash = creator_hash;
    vault.nonce = nonce;
    vault.index = index;
    vault.locked_at = Clock::get()?.unix_timestamp;

    // Transfer NFT to vault using CPI
    let cpi_accounts = crate::utils::transfer_compressed_nft(
        ctx.accounts.bubblegum_program.to_account_info(),
        ctx.accounts.tree_authority.to_account_info(),
        ctx.accounts.owner.to_account_info(),
        vault.key(),
        ctx.accounts.merkle_tree.to_account_info(),
        ctx.accounts.log_wrapper.to_account_info(),
        ctx.accounts.compression_program.to_account_info(),
        root,
        data_hash,
        creator_hash,
        nonce,
        index,
        ctx.remaining_accounts,
    )?;

    // Mint fraction tokens
    let fraction_amount = calculate_fraction_amount(&data_hash, &creator_hash);
    mint_to(
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
        fraction_amount,
    )?;

    msg!("cNFT locked in vault: {}", vault.key());
    Ok(())
}