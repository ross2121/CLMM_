use anchor_lang::{prelude::*, solana_program::system_instruction::transfer, system_program::Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface,TransferChecked,transfer_checked},
};

use crate::{accounts, error::CLMMError, state::{tick, Pool}, utils::{calculate_liquidity_amounts, tick_to_sqrt_price_x64, TICK_SPACING}};

#[derive(Accounts)]
pub struct Liquidity<'info>{
#[account(mut)]
    pub signer:Signer<'info>,
    pub minta:InterfaceAccount<'info, Mint>,
    pub mintb:InterfaceAccount<'info, Mint>,
    #[account(init,seeds=[b"lp",config.key().as_ref()],bump,payer=signer,mint::decimals=6,mint::authority=config)]
    pub lp_mint:InterfaceAccount<'info, Mint>,
    #[account(mut,associated_token::mint=minta,associated_token::authority=signer)]
    pub usertoken_account_a:InterfaceAccount<'info,TokenAccount>,
    #[account(mut,associated_token::mint=mintb,associated_token::authority=signer)]
    pub usertoken_account_b:InterfaceAccount<'info,TokenAccount>,
    #[account(init,associated_token::mint=minta,associated_token::authority=signer,payer=signer)]
    pub vaulta:InterfaceAccount<'info, TokenAccount>,
    #[account(init,seeds=[b"config",signer.key().as_ref()],bump,payer=signer,space=8+Pool::INIT_SPACE)]
    pub config:Account<'info,Pool>,
    #[account(init,associated_token::mint=mintb,associated_token::authority=signer,payer=signer)]
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
        let  pool=&ctx.accounts.config;
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

Ok(())
      }
}