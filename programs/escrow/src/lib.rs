pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3Dz2pbsazJTFFnBuEtN2RpAeRC3z1c6qmNn9Z6hYwrBG");

#[program]
pub mod escrow {
    use super::*;

    pub fn make(ctx: Context<Make>, id: u64, amount: u64, receive: u64) -> Result<()> {
        crate::instructions::make::handle_make(ctx, id, amount, receive)
    }

    pub fn take(ctx: Context<Take>, id: u64) -> Result<()> {
        crate::instructions::take::handle_take(ctx, id)
    }

    pub fn refund(ctx: Context<Refund>, id: u64) -> Result<()> {
        crate::instructions::refund::handle_refund(ctx, id)
    }
}
