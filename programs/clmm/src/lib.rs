use anchor_lang::prelude::*;
mod state;
pub mod instructions;
pub use instructions::*;
mod error;
mod utils;
declare_id!("BqGdgHyFoLxyrcgatBXCXqqUDqK1PUrRNbReAY1cxbp3");

#[program]
pub mod clmm {
    use super::*;

    pub fn init_pool(ctx: Context<InitializePool>, seed: u64, price: u64) -> Result<()> {
        InitializePool::initializepool(ctx, price, seed)
    }

    pub fn init_tick(ctx: Context<InitialTick>) -> Result<()> {
        InitialTick::initializetick(ctx)
    }

    pub fn add_liquidity(
        ctx: Context<Liquidity>,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    ) -> Result<()> {
        Liquidity::add_liqiudity(ctx, tick_lower, tick_upper, liquidity as u64)
    }

    pub fn withdraw_liquidity(
        ctx: Context<Withdraw>,
        tick_lower: i32,
        tick_upper: i32,
        liquidity_to_remove: u128,
    ) -> Result<()> {
        Withdraw::add_liqiudity(ctx, tick_lower, tick_upper, liquidity_to_remove as u64)
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        sqrt_price_limit: Option<u128>,
        min_amount_out: Option<u128>,
        a_to_b: bool,
    ) -> Result<()> {
        Swap::swap(ctx, amount_in, sqrt_price_limit, min_amount_out, a_to_b)
    }
}


