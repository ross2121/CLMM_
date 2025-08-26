use anchor_lang::prelude::*;
mod state;
mod instructions;
mod error;
mod utils;
declare_id!("BqGdgHyFoLxyrcgatBXCXqqUDqK1PUrRNbReAY1cxbp3");

#[program]
pub mod clmm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
