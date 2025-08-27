use std::iter::Product;

use anchor_lang::{prelude::*, solana_program::system_instruction::transfer, system_program::Transfer};
use anchor_spl::{
    associated_token::AssociatedToken, token::{burn, Burn}, token_interface::{mint_to, transfer_checked, Mint, MintTo, TokenAccount, TokenInterface, TransferChecked}
};

use crate::{accounts, error::CLMMError, state::{tick, Pool}, utils::{calculate_liquidity_amounts, integer_sqrt, tick_to_sqrt_price_x64, TICK_SPACING}};

#[derive(Accounts)]
pub struct Withdraw<'info>{
#[account(mut)]
    pub signer:Signer<'info>,
    pub minta:InterfaceAccount<'info, Mint>,
    pub mintb:InterfaceAccount<'info, Mint>,
    #[account(mut,seeds=[b"lp",config.key().as_ref()],bump)]
    pub lp_mint:InterfaceAccount<'info, Mint>,
    #[account(mut,associated_token::mint=minta,associated_token::authority=signer)]
    pub usertoken_account_a:InterfaceAccount<'info,TokenAccount>,
    #[account(mut,associated_token::mint=mintb,associated_token::authority=signer)]
    pub usertoken_account_b:InterfaceAccount<'info,TokenAccount>,
    #[account(init_if_needed,payer=signer,associated_token::mint=lp_mint,associated_token::authority=signer)]
     pub user_lp_account:InterfaceAccount<'info,TokenAccount> ,
    #[account(init,associated_token::mint=minta,associated_token::authority=config,payer=signer)]
    pub vaulta:InterfaceAccount<'info, TokenAccount>,
    #[account(mut,seeds=[b"config",config.seed.to_le_bytes().as_ref()],bump)]
    pub config:Account<'info,Pool>,
    #[account(mut,associated_token::mint=mintb,associated_token::authority=config)]
    pub  vault_b:InterfaceAccount<'info, TokenAccount>,
    #[account(mut,seeds=[b"tick",config.key().as_ref()],bump)]
    pub uppertick:Account<'info,tick>,
    #[account(mut,seeds=[b"tick",config.key().as_ref()],bump)]
    pub lowertick:Account<'info,tick>,
    pub system_program:Program<'info,System>,

    pub token_program:Interface<'info, TokenInterface>,
    pub associated_token_program:Program<'info,AssociatedToken>
}
impl <'info> Withdraw<'info> {
      pub fn add_liqiudity(ctx:Context<Withdraw>,lower_tick:i32,upper_tick:i32,liquidity:u64)->Result<()>{
        require!(upper_tick>lower_tick,CLMMError::TickMismatch);
        require!(liquidity>0,CLMMError::ZeroAmount);
        let  pool=&mut ctx.accounts.config;
        require!(pool.total_lp_issued>0,CLMMError::PoolEmpty);
        let lowertick=&mut ctx.accounts.lowertick;
        let uppertick=&mut ctx.accounts.uppertick;
        require!(lower_tick==lowertick.index,CLMMError::InvalidTickIndex);

        require!(upper_tick==uppertick.index,CLMMError::InvalidTickIndex);
        require!(pool.minta==ctx.accounts.minta.key(),CLMMError::InvalidTokenMint);
        require!(pool.mintb==ctx.accounts.mintb.key(),CLMMError::InvalidTokenMint);
        require!(lower_tick%TICK_SPACING==0 && upper_tick%TICK_SPACING==0,CLMMError::UnalignedTick);
        lowertick.liquidity=lowertick.liquidity.checked_add(liquidity as i128).ok_or(CLMMError::ArithmeticOverflow)?;
        uppertick.liquidity=uppertick.liquidity.checked_sub(liquidity as i128).ok_or(CLMMError::ArithmeticOverflow)?;
       let price_lower=tick_to_sqrt_price_x64(lower_tick)?;
       let price_uperr=tick_to_sqrt_price_x64(upper_tick)?;
       let (amounta,amountb)=calculate_liquidity_amounts(pool.sqrt_price, price_lower,price_uperr,liquidity as u128)?;
         let pool_balnce_a=ctx.accounts.vaulta.amount;
         let pool_balance_b=ctx.accounts.vault_b.amount;
         let lptoken=if pool_balnce_a>0 && pool_balance_b>0{
            let sharea=amounta.checked_mul(pool.total_lp_issued).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(pool_balnce_a).ok_or(CLMMError::ArithmeticOverflow)?;
            let shareb=amountb.checked_mul(pool.total_lp_issued).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(pool_balance_b).ok_or(CLMMError::ArithmeticOverflow)?;
            std::cmp::max(sharea, shareb)
         }else if pool_balnce_a>0{
            amounta.checked_mul(pool.total_lp_issued).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(pool_balnce_a).ok_or(CLMMError::ArithmeticOverflow)? as u64
         }else if pool_balance_b>0{
            amountb.checked_mul(pool.total_lp_issued).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(pool_balance_b).ok_or(CLMMError::ArithmeticOverflow)? as u64
         }else{
return  Err(CLMMError::PoolEmpty.into());
         };
   require!(lptoken>ctx.accounts.user_lp_account.amount,CLMMError::InsufficientFundsInPool); 
   lowertick.liquidity=lowertick.liquidity.checked_sub(liquidity as i128).ok_or(CLMMError::ArithmeticOverflow)?;
   uppertick.liquidity=uppertick.liquidity.checked_add(liquidity as i128).ok_or(CLMMError::ArithmeticOverflow)?;
   if lower_tick<=pool.current_tick && pool.current_tick<=upper_tick{
    pool.active_liqiudity=pool.active_liqiudity.checked_sub(liquidity as u128).ok_or(CLMMError::ArithmeticOverflow)?;
   }
   let seed=pool.seed.to_be_bytes();
   let seeds:&[&[u8]]=&[b"config",seed.as_ref(),&[pool.bump]];
   let signer_seed=&[seeds];
   if amounta!=0{
    let account=TransferChecked{
        from:ctx.accounts.vaulta.to_account_info(),
        to:ctx.accounts.usertoken_account_a.to_account_info(),
        authority:pool.to_account_info(),
        mint:ctx.accounts.minta.to_account_info()
    };
    let cpi_ctx=CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), account, signer_seed);
    transfer_checked(cpi_ctx, amounta, ctx.accounts.minta.decimals)?;
   }
   if amountb!=0{
    let account=TransferChecked{
        from:ctx.accounts.vault_b.to_account_info(),
        to:ctx.accounts.usertoken_account_b.to_account_info(),
        authority:pool.to_account_info(),
        mint:ctx.accounts.mintb.to_account_info()
    };
    let cpi_ctx=CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), account, signer_seed);
    transfer_checked(cpi_ctx, amounta, ctx.accounts.mintb.decimals)?;
   }
   let account=Burn{
    mint:ctx.accounts.lp_mint.to_account_info(),
    from:ctx.accounts.user_lp_account.to_account_info(),
    authority:pool.to_account_info()
   };
   let cpi_ctx=CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), account, signer_seed);
   burn(cpi_ctx, lptoken)?;
   pool.active_liquidity=pool.active_liqiudity.checked_sub(lptoken as u128).ok_or(CLMMError::ArithmeticOverflow)?;

Ok(())
      }
}