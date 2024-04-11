//! A very simple AMM protocol.
use anchor_lang::prelude::*;
use anchor_spl::token;

declare_id!("9Ld1EJjQtqqvR8qac66JTwc375pwdvpPWqdmzLK32FwR");

/// token_0_reserves * token_1_reserves = invariant
#[account]
pub struct Pool {
    pub token_0: Pubkey,
    pub token_1: Pubkey,
    pub token_0_reserves: u64,
    pub token_1_reserves: u64,
}

#[program]
pub mod amm {
    use super::*;

    pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
        ctx.accounts.pool.set_inner(Pool {
            token_0: ctx.accounts.token_0.key(),
            token_1: ctx.accounts.token_1.key(),
            token_0_reserves: 0,
            token_1_reserves: 0,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        space = 8 + std::mem::size_of::<Pool>(),
        payer = payer,
    )]
    pub pool: Account<'info, Pool>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        constraint = token_0.key() != token_1.key() @ AmmError::MatchingTokenMints
    )]
    pub token_0: Account<'info, token::Mint>,
    pub token_1: Account<'info, token::Mint>,
}

#[error_code]
pub enum AmmError {
    #[msg("Token 0 cannot be the same as token 1")]
    MatchingTokenMints,
}
