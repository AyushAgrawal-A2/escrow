use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct Refund<'info> {
    #[account(mut)]
    maker: Signer<'info>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = maker,
        has_one = mint_a
    )]
    escrow: Account<'info, Escrow>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    escrow_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    maker_a_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mint::token_program = token_program
    )]
    mint_a: InterfaceAccount<'info, Mint>,

    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

pub fn handle_refund(ctx: Context<Refund>, id: u64) -> Result<()> {
    let seeds = [
        ESCROW_SEED,
        ctx.accounts.maker.key.as_ref(),
        &id.to_le_bytes(),
        &[ctx.accounts.escrow.bump],
    ];

    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            token_interface::TransferChecked {
                from: ctx.accounts.escrow_vault.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                to: ctx.accounts.maker_a_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            &[&seeds[..]],
        ),
        ctx.accounts.escrow_vault.amount,
        ctx.accounts.mint_a.decimals,
    )?;

    token_interface::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        token_interface::CloseAccount {
            account: ctx.accounts.escrow_vault.to_account_info(),
            destination: ctx.accounts.maker.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        &[&seeds[..]],
    ))?;

    Ok(())
}
