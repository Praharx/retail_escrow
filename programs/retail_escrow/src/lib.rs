use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("BjNAQd5f4YCY963A1CNscxGLTKFToSeyCoSXxh6QjzgS");

#[program]
pub mod retail_escrow {
    use super::*;

    pub fn initialize_escrow(
        ctx: Context<InitializeEscrow>,
        escrow_id: u64,
        amount: u64,
        retailer_key: Pubkey
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.buyer = ctx.accounts.buyer.key();
        escrow.retailer = retailer_key;
        escrow.escrow_id = escrow_id;
        escrow.amount = amount;
        escrow.created_at = Clock::get()?.unix_timestamp;
        escrow.state = EscrowState::AwaitingDelivery;
        escrow.is_completed = false;

        // Transfer funds from buyer token account to escrow token account
        let transfer_instruction = Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info()
        };

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
            ),
            amount
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(escrow_id: u64)]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        init,
        payer = buyer,
        space = 8 + 32 + 32 + 8 + 8 + 1+ 8 + 1,
        seeds = [b"escrow", buyer.key().as_ref(), &escrow_id.to_le_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    pub retailer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>
}

#[account]
pub struct Escrow {
    pub buyer: Pubkey,
    pub retailer: Pubkey,
    pub escrow_id: u64,
    pub amount: u64,
    pub state: EscrowState, 
    pub created_at: i64,
    pub is_completed: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum EscrowState { // usually enum's options take up 1 byte of space
    AwaitingDelivery, // this will be set by the retailer
    AwaitingConfirmation, // this will be set by the buyer
    Completed
}


