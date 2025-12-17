use anchor_lang::prelude::*;

mod constant;
mod instructions;
mod state;
mod util;
mod tests;

use instructions::*;

declare_id!("8wjJau9UuUBHBWiafvh2svxp4rCqkDpcUa1j13EdYh5C");

#[program]
pub mod sol_4626_vault {
    use super::*;
    use crate::instructions::initialize;
    use crate::util::{is_valid_ticker, Errors};

    pub fn initialize(ctx: Context<Initialize>, ticker: [u8; 16]) -> Result<()> {
        require!(is_valid_ticker(&ticker), Errors::InvalidTicker);
        initialize::handle(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, ticker: [u8; 16]) -> Result<()> {
        require!(is_valid_ticker(&ticker), Errors::InvalidTicker);
        deposit::handle(ctx, amount, &ticker)
    }

    pub fn allocate(ctx: Context<Allocate>, amount: u64, ticker: [u8; 16]) -> Result<()> {
        require!(is_valid_ticker(&ticker), Errors::InvalidTicker);
        allocate::handle(ctx, amount, &ticker)
    }
}
