use crate::constant::{VAULT_AUTHORITY_SEED};
use crate::state::Vault;
use crate::util::Errors;
use anchor_lang::prelude::*;
use anchor_lang::Accounts;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

/// Allocate accounts:
/// - signer: vault admin
/// - base_asset_mint: vault's base asset mint
/// - vault_base_asset_ata: vault's ATA for base assets
/// - vault: vault PDA
/// - target_ata: external target ATA to allocate assets to
/// - token_program
/// - system_program
#[derive(Accounts)]
#[instruction(amount: u64, ticker: [u8; 16])]
pub struct Allocate<'info> {
    #[account(mut)]
    admin: Signer<'info>,
    #[account(mut,
        has_one = base_asset_mint,
        has_one = vault_authority,
        has_one = vault_base_asset_ata,
        has_one = token_program,
        has_one = admin
    )]
    vault: AccountLoader<'info, Vault>,
    /// CHECK: vault authority checked (has_one)
    #[account(mut)]
    vault_authority: AccountInfo<'info>,
    #[account(
        mint::token_program = token_program,
    )]
    base_asset_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut)]
    vault_base_asset_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        token::mint = base_asset_mint,
        token::token_program = token_program,
    )]
    target_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

/// Moves base assets from the vault's ATA to an external target ATA.
/// The vault's accounting keeps total assets unchanged because funds
/// are only being relocated (e.g., allocated to an external yield strategy),
/// This is just a poc, better way of doing that is to manage idle and in_use assets
pub fn handle(ctx: Context<Allocate>, amount: u64) -> Result<()> {
    let vlt = ctx.accounts.vault.load()?;

    require!(amount > 0, Errors::InvalidAmount);
    require!(vlt.deposit_paused == 0, Errors::AllocatePaused);
    require!(
        amount <= ctx.accounts.vault_base_asset_ata.amount,
        Errors::InsufficientBaseAssetBalance
    );

    msg!(
        "allocating {} base assets from vault {} to target ATA {}",
        amount,
        ctx.accounts.vault.key(),
        ctx.accounts.target_ata.key()
    );

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.vault_base_asset_ata.to_account_info(),
        mint: ctx.accounts.base_asset_mint.to_account_info(),
        to: ctx.accounts.target_ata.to_account_info(),
        authority: ctx.accounts.vault.to_account_info(),
    };

    let vlt_address = ctx.accounts.vault.key();
    let vlt_auth_seeds: &[&[&[u8]]] = &[&[
        VAULT_AUTHORITY_SEED.as_bytes(),
        vlt_address.as_ref(),
        &[vlt.vault_authority_bump],
    ]];
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        transfer_accounts,
        vlt_auth_seeds,
    );

    transfer_checked(transfer_ctx, amount, ctx.accounts.base_asset_mint.decimals)?;

    emit!(AllocateEvent {
        vault: ctx.accounts.vault.key(),
        target_ata: ctx.accounts.target_ata.key(),
        amount,
    });

    Ok(())
}

#[event]
pub struct AllocateEvent {
    pub vault: Pubkey,
    pub target_ata: Pubkey,
    pub amount: u64,
}
