use std::iter::Product;

use anchor_lang::{prelude::*, solana_program::system_instruction::transfer, system_program::Transfer};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface,TransferChecked,transfer_checked,MintTo,mint_to},
};

use crate::{accounts, error::CLMMError, state::{tick, Pool}, utils::{calculate_liquidity_amounts, compute_swap_step, integer_sqrt, tick_to_sqrt_price_x64, TICK_SPACING}};

#[derive(Accounts)]
pub struct Swap<'info>{
#[account(mut)]
    pub useraccount:Signer<'info>,

     pub  pooladmint:SystemAccount<'info>,
    pub minta:InterfaceAccount<'info, Mint>,
    pub mintb:InterfaceAccount<'info, Mint>,
    #[account(init_if_needed,associated_token::mint=minta,associated_token::authority=useraccount,payer=useraccount)]
    pub usertoken_account_a:InterfaceAccount<'info,TokenAccount>,
    #[account(init_if_needed,associated_token::mint=mintb,associated_token::authority=useraccount,payer=useraccount)]
    pub usertoken_account_b:InterfaceAccount<'info,TokenAccount>,
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
impl <'info> Swap<'info> {
    pub fn swap(ctx:Context<Swap>,amount_in:u64,sqrt_price:Option<u128>,min_amount:Option<u128>,a_to_b:bool)->Result<()>{
        require!(amount_in>0,CLMMError::ZeroAmount);
        require!(!ctx.remaining_accounts.is_empty(),CLMMError::MissingTickAccounts);
        let  pool=&mut ctx.accounts.config;
        let mut liquidity=pool.active_liqiudity;
        require!(pool.minta==ctx.accounts.minta.key(),CLMMError::InvalidTokenMint);
        require!(pool.mintb==ctx.accounts.mintb.key(),CLMMError::InvalidTokenMint);
        require!(pool.active_liqiudity>0,CLMMError::InsufficientFundsInPool);
        //since the tick accounts are variable that is why passing in remainig account
        let mut ticks=vec![];
        for account in ctx.remaining_accounts.iter(){
            let data=account.data.borrow_mut();
            let mut tick_data=&data[8..];
            let tick:tick=tick::try_deserialize(&mut tick_data)?;
            ticks.push((account.clone(),tick));
        }
    let mut sqrt_price=sqrt_price.unwrap_or_else(|| if  a_to_b{1} else {
        u128::MAX
    });
    let mut remainig_amount=pool.active_liqiudity;
    let mut total_amount_in:u128=0;
    let mut total_amount_out:u128=0;

     for (tick_info,tick) in ticks{
        let next_sqrt=tick_to_sqrt_price_x64(tick.index)?;
        if (a_to_b && next_sqrt < sqrt_price)
        || (!a_to_b && next_sqrt > sqrt_price)
    {
        break;
    }
 
    let (sqrt_new,computed_amount_in,computed_amount_out)=compute_swap_step(
        sqrt_price, next_sqrt, pool.active_liqiudity,amount_in as u128, a_to_b)?;

        sqrt_price=sqrt_new;
        remainig_amount=remainig_amount.checked_sub(computed_amount_in).ok_or(CLMMError::ArithmeticOverflow)?;
        total_amount_in=total_amount_in.checked_add(computed_amount_in).ok_or(CLMMError::ArithmeticOverflow)?;
        total_amount_out=total_amount_out.checked_add(computed_amount_out).ok_or(CLMMError::ArithmeticOverflow)?;
        if sqrt_price==sqrt_new{
            pool.current_tick=tick.index;
            if a_to_b{
                liquidity=liquidity.checked_sub(tick.liquidity as u128).ok_or(CLMMError::ArithmeticOverflow)?;
            }else{
                liquidity=liquidity.checked_add(tick.liquidity as u128).ok_or(CLMMError::ArithmeticOverflow)?;
            }

        }

}     
   
   let total_amount_in:u64=total_amount_in.try_into().map_err(|_|CLMMError::AmountTooLarge)?;
   let total_amount_out:u64=total_amount_out.try_into().map_err(|_|CLMMError::AmountTooLarge)?;
    if let Some(min_out)=min_amount{
        require!(total_amount_out>=min_out as u64 ,CLMMError::SlippageExceeded);
    }  
  require!(total_amount_out>0,CLMMError::ZeroSwapOutput);
  pool.sqrt_price=sqrt_price;
  pool.active_liqiudity=liquidity;
  let seed=pool.seed.to_be_bytes();
let seeds:&[&[u8]]=&[b"config",seed.as_ref(),&[pool.bump]];
let signer_seed=&[seeds];
if a_to_b{
    let account=TransferChecked{
        from:ctx.accounts.usertoken_account_a.to_account_info(),
        to:ctx.accounts.vaulta.to_account_info(),
        authority:ctx.accounts.useraccount.to_account_info(),
        mint:ctx.accounts.minta.to_account_info()
    };
    let cpi_ctx=CpiContext::new(ctx.accounts.token_program.to_account_info(), account);
    transfer_checked(cpi_ctx, total_amount_in, ctx.accounts.minta.decimals)?;
    let account=TransferChecked{
        to:ctx.accounts.usertoken_account_b.to_account_info(),
        from:ctx.accounts.vault_b.to_account_info(),
        authority:ctx.accounts.config.to_account_info(),
        mint:ctx.accounts.mintb.to_account_info()
    };
    let cpi_ctx=CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), account,signer_seed);
    transfer_checked(cpi_ctx, total_amount_out, ctx.accounts.mintb.decimals)?;
}
        Ok(())

    }
}