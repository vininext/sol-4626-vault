use crate::constant::{SHARES_MINT_SEED, VAULT_AUTHORITY_SEED};
use crate::state::Vault;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

#[derive(Accounts)]
#[instruction()]
pub struct Initialize<'info> {
    #[account(mut)]
    admin: Signer<'info>,
    #[account(zero)]
    vault: AccountLoader<'info, Vault>,
    /// CHECK: PDA used only as signing authority
    #[account(
        seeds = [VAULT_AUTHORITY_SEED.as_bytes(), vault.key().as_ref()],
        bump
    )]
    vault_authority: AccountInfo<'info>,
    #[account(
        mint::token_program = token_program
    )]
    base_asset_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = base_asset_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program
    )]
    vault_base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init,
        payer = admin,
        mint::decimals = base_asset_mint.decimals,
        mint::authority = vault_authority,
        seeds = [SHARES_MINT_SEED.as_bytes(), vault_authority.key().as_ref()],
        bump
    )]
    shares_mint: InterfaceAccount<'info, Mint>,
    token_program: Interface<'info, TokenInterface>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
}

pub fn handle(ctx: Context<Initialize>) -> Result<()> {
    msg!(
        "initializing vault address: {} vault authority {} shares_mint: {} base_asset_mint: {}",
        ctx.accounts.vault.key(),
        ctx.accounts.vault_authority.key(),
        ctx.accounts.shares_mint.key(),
        ctx.accounts.base_asset_mint.key()
    );

    let vlt = &mut ctx.accounts.vault.load_init()?;

    let admin = ctx.accounts.admin.key();
    let vault_authority = ctx.accounts.vault_authority.key();
    let shares_mint = ctx.accounts.shares_mint.key();
    let shares_mint_decimals = ctx.accounts.shares_mint.decimals;
    let base_asset_mint = ctx.accounts.base_asset_mint.key();
    let vault_base_asset_ata = ctx.accounts.vault_base_asset_ata.key();
    let token_program = ctx.accounts.token_program.key();

    vlt.initialize(
        admin,
        vault_authority,
        shares_mint,
        base_asset_mint,
        vault_base_asset_ata,
        token_program,
        shares_mint_decimals,
        ctx.bumps.vault_authority,
        ctx.bumps.shares_mint,
    )?;

    emit!(InitializeEvent {
        vault: ctx.accounts.vault.key(),
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
