use std::iter::Product;

use anchor_lang::{prelude::*, solana_program::system_instruction::transfer, system_program::Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface,TransferChecked,transfer_checked,MintTo,mint_to},
};

use crate::{accounts, error::CLMMError, state::{tick, Pool}, utils::{calculate_liquidity_amounts, integer_sqrt, tick_to_sqrt_price_x64, TICK_SPACING}};

#[derive(Accounts)]
pub struct Liquidity<'info>{
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
    #[account(mut,associated_token::mint=minta,associated_token::authority=config)]
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
impl <'info> Liquidity<'info> {
      pub fn add_liqiudity(ctx:Context<Liquidity>,lower_tick:i32,upper_tick:i32,liquidity:u64)->Result<()>{
        let  pool=&mut ctx.accounts.config;
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
    if amounta!=0{
        let account=TransferChecked{
            from:ctx.accounts.usertoken_account_a.to_account_info(),
            to:ctx.accounts.vaulta.to_account_info(),
            mint:ctx.accounts.minta.to_account_info(),
            authority:ctx.accounts.signer.to_account_info()
        };
        let cpicontext=CpiContext::new(ctx.accounts.token_program.to_account_info(), account);
    transfer_checked(cpicontext, amounta, ctx.accounts.minta.decimals)?;
    }
    if amountb!=0{
        let account=TransferChecked{
            from:ctx.accounts.usertoken_account_b.to_account_info(),
            to:ctx.accounts.vault_b.to_account_info(),
            mint:ctx.accounts.mintb.to_account_info(),
            authority:ctx.accounts.signer.to_account_info()
        };
        let cpicontext=CpiContext::new(ctx.accounts.token_program.to_account_info(), account);
    transfer_checked(cpicontext, amounta, ctx.accounts.mintb.decimals)?;
    }
    let mintamount=if pool.total_lp_issued==0{
        if amounta>0 && amountb>0{
            let product=(amounta).checked_mul(amountb).ok_or(CLMMError::ArithmeticOverflow)?;
            integer_sqrt(product as u128)
        }else {
            std::cmp::max(amounta,amountb)
        }
    }else{
        let poolbalancea=ctx.accounts.vaulta.amount;
        let poolbalanceb=ctx.accounts.vault_b.amount;
        if poolbalancea==0 && poolbalanceb==0{
            return Err(CLMMError::PoolEmpty.into());
        } 
         let sharea=if poolbalancea>0{
            (amounta as u128).checked_mul(pool.total_lp_issued as u128).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(poolbalancea as u128).ok_or(CLMMError::ArithmeticOverflow)?
         }else{
            0
           };  let shareb=if poolbalanceb>0{
            (amountb as u128).checked_mul(pool.total_lp_issued as u128).ok_or(CLMMError::ArithmeticOverflow)?.checked_div(poolbalanceb as u128).ok_or(CLMMError::ArithmeticOverflow)?
         }else{
            0
           };

        std::cmp::min(sharea as u64,shareb as u64)


    };
    let seed=pool.seed.to_be_bytes();
   let seeds:&[&[u8]]=&[b"config",seed.as_ref(),&[pool.bump]];
   let signer_seed=&[seeds];
    pool.total_lp_issued=pool.total_lp_issued.checked_add(mintamount).ok_or(CLMMError::ArithmeticOverflow)?;
     let account=MintTo{
          authority:ctx.accounts.config.to_account_info(),
          to:ctx.accounts.user_lp_account.to_account_info(),
          mint:ctx.accounts.lp_mint.to_account_info()
     };
     let cpi_context=CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), account,signer_seed);
      mint_to(cpi_context, mintamount)?;
Ok(())
      }
}