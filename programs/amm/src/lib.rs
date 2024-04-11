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
    pub pda_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Eq)]
pub enum SwapDirection {
    Token0ToToken1,
    Token1ToToken0,
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
            pda_bump: ctx.bumps.pool,
        });

        Ok(())
    }

    pub fn provide_liquidity(
        ctx: Context<ProvideLiquidity>,
        token_0_max: u64,
        token_1_max: u64,
    ) -> Result<()> {
        let pool = &mut ctx.accounts.pool;

        if pool.token_0_reserves == 0 {
            // if the pool doesn't currently have liquidity, transfer tokens in the provided proportion

            for (user_token_account, token_vault, amount) in [
                (
                    &ctx.accounts.user_token_0_account,
                    &ctx.accounts.token_0_vault,
                    token_0_max,
                ),
                (
                    &ctx.accounts.user_token_1_account,
                    &ctx.accounts.token_1_vault,
                    token_1_max,
                ),
            ] {
                let cpi_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: user_token_account.to_account_info(),
                        to: token_vault.to_account_info(),
                        authority: ctx.accounts.signer.to_account_info(),
                    },
                );
                token::transfer(cpi_ctx, amount)?;
            }

            pool.token_0_reserves = token_0_max;
            pool.token_1_reserves = token_1_max;
        }

        Ok(())
    }

    pub fn swap(ctx: Context<Swap>, direction: SwapDirection, amount_in: u64) -> Result<()> {
        let pool = &mut ctx.accounts.pool;

        let (user_from, user_to, vault_to, vault_from) = match direction {
            SwapDirection::Token0ToToken1 => (
                &ctx.accounts.user_token_0_account,
                &ctx.accounts.user_token_1_account,
                &ctx.accounts.token_0_vault,
                &ctx.accounts.token_1_vault,
            ),
            SwapDirection::Token1ToToken0 => (
                &ctx.accounts.user_token_1_account,
                &ctx.accounts.user_token_0_account,
                &ctx.accounts.token_1_vault,
                &ctx.accounts.token_0_vault,
            ),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: user_from.to_account_info(),
                to: vault_to.to_account_info(),
                authority: ctx.accounts.signer.to_account_info(),
            },
        );

        token::transfer(cpi_ctx, amount_in)?;

        // x * y = k
        // (x + Δx)(y - Δy) = k
        // FROM THIS WE CAN DERIVE
        // (y - Δy) = k / (x + Δx)
        // -Δy = [k / (x + Δx)] - y
        // Δy = -[k / (x + Δx)] + y
        // Δy = y - [k / (x + Δx)]
        // WE CAN ALSO DERIVE
        // (x + Δx) = k / (y - Δy)
        // Δx = [k / y - Δy] - x
        
        let x = u128::from(pool.token_0_reserves);
        let y = u128::from(pool.token_1_reserves);
        let k = x * y;
        
        let amount_out = if direction == SwapDirection::Token0ToToken1 {
            let delta_x = u128::from(amount_in);
            let delta_y = y - (k / (x + delta_x));

            u64::try_from(delta_y).unwrap()
        } else {
            let delta_y = u128::from(amount_in);
            let delta_x = (k / (y + delta_y)) - x;

            u64::try_from(delta_x).unwrap()
        };

        let token_0 = pool.token_0.key();
        let token_1 = pool.token_1.key();
        let seeds = &[b"pool", token_0.as_ref(), token_1.as_ref(), &[pool.pda_bump]];
        let signer = &[&seeds[..]];
        
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: vault_from.to_account_info(),
                to: user_to.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer
        );

        token::transfer(cpi_ctx, amount_out)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(
        init,
        seeds = [b"pool", token_0.key().as_ref(), token_1.key().as_ref()],
        bump,
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

#[derive(Accounts)]
pub struct ProvideLiquidity<'info> {
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut, token::mint = pool.token_0)]
    pub user_token_0_account: Account<'info, token::TokenAccount>,
    #[account(mut, token::mint = pool.token_1)]
    pub user_token_1_account: Account<'info, token::TokenAccount>,
    pub signer: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = pool.token_0,
        associated_token::authority = pool
    )]
    pub token_0_vault: Account<'info, token::TokenAccount>,
    #[account(
        mut,
        associated_token::mint = pool.token_1,
        associated_token::authority = pool
    )]
    pub token_1_vault: Account<'info, token::TokenAccount>,
    pub token_program: Program<'info, token::Token>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    #[account(mut, token::mint = pool.token_0)]
    pub user_token_0_account: Account<'info, token::TokenAccount>,
    #[account(mut, token::mint = pool.token_1)]
    pub user_token_1_account: Account<'info, token::TokenAccount>,
    #[account(
        mut,
        associated_token::mint = pool.token_0,
        associated_token::authority = pool
    )]
    pub token_0_vault: Account<'info, token::TokenAccount>,
    #[account(
        mut,
        associated_token::mint = pool.token_1,
        associated_token::authority = pool
    )]
    pub token_1_vault: Account<'info, token::TokenAccount>,
    pub token_program: Program<'info, token::Token>,
}

#[error_code]
pub enum AmmError {
    #[msg("Token 0 cannot be the same as token 1")]
    MatchingTokenMints,
}
