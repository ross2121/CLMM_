use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{state::Pool, utils::{price_to_sqrt_price_x64, sqrt_price_x64_to_tick}};

#[derive(Accounts)]
#[instruction(seeds:u64)]
pub struct InitializePool<'info>{
#[account(mut)]
pub signer:Signer<'info>,
pub minta:InterfaceAccount<'info, Mint>,
pub mintb:InterfaceAccount<'info, Mint>,
#[account(init,seeds=[b"lp",config.key().as_ref()],bump,payer=signer,mint::decimals=6,mint::authority=config)]
pub lp_mint:InterfaceAccount<'info, Mint>,
#[account(init,associated_token::mint=minta,associated_token::authority=config,payer=signer)]
pub vaulta:InterfaceAccount<'info, TokenAccount>,
#[account(init,seeds=[b"config",seeds.to_le_bytes().as_ref()],bump,payer=signer,space=8+Pool::INIT_SPACE)]
pub config:Account<'info,Pool>,
#[account(init,associated_token::mint=mintb,associated_token::authority=config,payer=signer)]
pub  vault_b:InterfaceAccount<'info, TokenAccount>,
pub system_program:Program<'info,System>,
pub token_program:Interface<'info, TokenInterface>,
pub associated_token_program:Program<'info,AssociatedToken>
}
impl <'info> InitializePool<'info> {
      pub fn initializepool(ctx:Context<InitializePool>,price:u64,seed:u64)->Result<()>{
             let curr_sqrt_price=price_to_sqrt_price_x64(price)?;
             let current_tick=sqrt_price_x64_to_tick(curr_sqrt_price);
             let  pool=&mut ctx.accounts.config;
             pool.minta=ctx.accounts.minta.key();
             pool.mintb=ctx.accounts.mintb.key();
             pool.lp_mint=ctx.accounts.lp_mint.key();
              pool.bump=ctx.bumps.config;
              pool.total_lp_issued=0;
              pool.active_liqiudity=0;
              pool.current_tick=current_tick.unwrap();
              pool.active_liqiudity=0;
              pool.sqrt_price=curr_sqrt_price;
              pool.seed=seed;
             Ok(())

      }
}