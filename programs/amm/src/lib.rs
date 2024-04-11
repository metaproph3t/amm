use anchor_lang::prelude::*;

declare_id!("9Ld1EJjQtqqvR8qac66JTwc375pwdvpPWqdmzLK32FwR");

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
