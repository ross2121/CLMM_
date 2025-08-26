use core::str;

use anchor_lang::prelude::*;

use crate::{state::{tick, Pool}, utils::tick_to_sqrt_price_x64};

#[derive(Accounts)]
pub struct InitialTick<'info>{
   #[account(mut)]
    pub signer:Signer<'info>,
    #[account(mut,seeds=[b"config",signer.key().as_ref()],bump=config.bump)]
    pub config:Account<'info,Pool>,
    #[account(init,seeds=[b"tick",config.key().as_ref()],bump,space=8+tick::INIT_SPACE,payer=signer)]
    pub  tick:Account<'info,tick>,
    pub system_program:Program<'info,System>
}
impl <'info> InitialTick<'info>{
    pub fn initializetick(ctx:Context<InitialTick>)->Result<()>{
        let tick=&mut ctx.accounts.tick;
        let sqrt_price_x64=tick_to_sqrt_price_x64(tick.index)?;
        tick.index=tick.index;
        tick.sqrt_price_x64=sqrt_price_x64;
        tick.liquidity=0;
        tick.bump=ctx.bumps.tick;
Ok(())
    }
}