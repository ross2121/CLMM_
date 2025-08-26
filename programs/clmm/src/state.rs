use anchor_lang::prelude::*;
#[derive(Debug,InitSpace)]
#[account]
pub struct Pool{
    pub minta:Pubkey,
    pub mintb:Pubkey,
    pub lp_mint:Pubkey,
    pub pool_authority:Pubkey,
    pub sqrt_price:u128,
    pub active_liquidity:u128,
    pub total_lp_liqidity:u128,
    pub active_liqiudity:u128,
    pub total_lp_issued:u64,
    pub current_tick:i32,
    pub bump:u8,
    pub padding:[u8;3],
}
#[account]
pub struct  tick{
    pub sqrt_price_x64:u128,
    pub liquidity:i128,
    pub index:i32,
    pub bump:u8
}