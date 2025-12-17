use crate::constant::{SHARES_MINT_SEED, VAULT_SEED};
use crate::state::Vault;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

/// Initialize accounts:
/// - admin: vault admin (payer)
/// - vault: vault PDA
/// - base_asset_mint: base token asset mint
/// - vault_base_asset_ata: vault's ATA for base assets
/// - shares_mint: vault's shares mint
/// - token_program
/// - associated_token_program
/// - system_program
#[derive(Accounts)]
#[instruction(ticker: [u8;16])]
pub struct Initialize<'info> {
    #[account(mut)]
    admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + Vault::MAX_SIZE,
        seeds = [VAULT_SEED.as_bytes(), &ticker[..]],
        bump
    )]
    vault: Account<'info, Vault>,
    #[account(
        mint::token_program = token_program
    )]
    base_asset_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = base_asset_mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program
    )]
    vault_base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = admin,
        mint::decimals = base_asset_mint.decimals,
        mint::authority = vault.key(),
        seeds = [SHARES_MINT_SEED.as_bytes(), vault.key().as_ref()],
        bump
    )]
    shares_mint: InterfaceAccount<'info, Mint>,
    token_program: Interface<'info, TokenInterface>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

/// Initializes the vault account with the provided admin, shares mint, and base asset mint.
/// Accepts ticker. One vault ticker per program.
pub fn handle(ctx: Context<Initialize>) -> Result<()> {
    msg!(
        "initializing vault address: {} shares_mint: {} base_asset_mint: {}",
        ctx.accounts.vault.key(),
        ctx.accounts.shares_mint.key(),
        ctx.accounts.base_asset_mint.key()
    );

    let vlt = &mut ctx.accounts.vault;

    let admin = ctx.accounts.admin.key();
    let shares_mint = ctx.accounts.shares_mint.key();
    let base_asset_mint = ctx.accounts.base_asset_mint.key();

    vlt.initialize(admin, shares_mint, base_asset_mint, ctx.bumps.vault)?;

    emit!(InitializeEvent {
        vault: vlt.key(),
        admin,
        shares_mint,
        base_asset_mint,
    });

    Ok(())
}

#[event]
pub struct InitializeEvent {
    pub vault: Pubkey,
    pub admin: Pubkey,
    pub shares_mint: Pubkey,
    pub base_asset_mint: Pubkey,
}
