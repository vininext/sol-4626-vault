use anchor_lang::prelude::*;

mod instructions;
mod libraries;
mod state;
mod util;

use instructions::*;

declare_id!("8wjJau9UuUBHBWiafvh2svxp4rCqkDpcUa1j13EdYh5C");

#[program]
pub mod sol_4626_vault {
    use super::*;
    use crate::instructions::initialize;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::handle(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit::handle(ctx, amount)
    }

    pub fn allocate(ctx: Context<Allocate>, amount: u64) -> Result<()> {
        allocate::handle(ctx, amount)
    }
}
