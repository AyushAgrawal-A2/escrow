use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, AssociatedToken},
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct Take<'info> {
    #[account(mut)]
    taker: Signer<'info>,

    #[account(mut)]
    /// CHECK: checked in escrow's has_one constraint
    maker: UncheckedAccount<'info>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, maker.key().as_ref(), id.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
    )]
    escrow: Account<'info, Escrow>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    escrow_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: create_idempotent in instruction handler
    taker_a_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    taker_b_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    /// CHECK: create_idempotent in instruction handler
    maker_b_ata: UncheckedAccount<'info>,

    #[account(
        mint::token_program = token_program
    )]
    mint_a: InterfaceAccount<'info, Mint>,

    #[account(
        mint::token_program = token_program
    )]
    mint_b: InterfaceAccount<'info, Mint>,

    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

pub fn handle_take(ctx: Context<Take>, id: u64) -> Result<()> {
    associated_token::create_idempotent(CpiContext::new(
        ctx.accounts.associated_token_program.key(),
        associated_token::Create {
            payer: ctx.accounts.taker.to_account_info(),
            associated_token: ctx.accounts.maker_b_ata.to_account_info(),
            authority: ctx.accounts.maker.to_account_info(),
            mint: ctx.accounts.mint_b.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
    ))?;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.key(),
            token_interface::TransferChecked {
                from: ctx.accounts.taker_b_ata.to_account_info(),
                mint: ctx.accounts.mint_b.to_account_info(),
                to: ctx.accounts.maker_b_ata.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
            },
        ),
        ctx.accounts.escrow.receive,
        ctx.accounts.mint_b.decimals,
    )?;

    let seeds = [
        ESCROW_SEED,
        ctx.accounts.maker.key.as_ref(),
        &id.to_le_bytes(),
        &[ctx.accounts.escrow.bump],
    ];

    associated_token::create_idempotent(CpiContext::new(
        ctx.accounts.associated_token_program.key(),
        associated_token::Create {
            payer: ctx.accounts.taker.to_account_info(),
            associated_token: ctx.accounts.taker_a_ata.to_account_info(),
            authority: ctx.accounts.taker.to_account_info(),
            mint: ctx.accounts.mint_a.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        },
    ))?;
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            token_interface::TransferChecked {
                from: ctx.accounts.escrow_vault.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                to: ctx.accounts.taker_a_ata.to_account_info(),
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
