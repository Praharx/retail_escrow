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
        escrow.state = EscrowState::AwaitingDelivery;

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


    pub fn confirm_delivery(ctx: Context<ConfirmDelivery>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;

        require!(escrow.state == EscrowState::AwaitingDelivery, EscrowError::InvalidEscrowState);

        escrow.state = EscrowState::AwaitingConfirmation;
        escrow.delivery_confirmed_at = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn confirm_receipt(ctx: Context<ConfirmReceipt>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let current_time = Clock::get()?.unix_timestamp;
        let amount = escrow.amount;
      
        require!(escrow.state == EscrowState::AwaitingConfirmation, EscrowError::InvalidEscrowState);
        require!(
            current_time <= escrow.delivery_confirmed_at + 604800, // 7 days in seconds(Buyer should confirm within the 7 days window about product confirmation),
            EscrowError::ConfirmationPeriodExpired
        );

        let seeds= &[
            b"escrow".as_ref(),
            &escrow.escrow_id.to_le_bytes(),
            &[ctx.bumps.escrow]
        ];        

        // Transfer funds from escrow token account to retailer token account
        let transfer_instruction = Transfer {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.retailer_token_account.to_account_info(),
            authority:ctx.accounts.escrow.to_account_info()
        };

        let escrow = &mut ctx.accounts.escrow;

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                transfer_instruction,
                &[seeds]
            ),
            amount
        )?;

        escrow.state = EscrowState::Completed;

        Ok(())
    } 

    pub fn auto_release(ctx: Context<AutoRelease>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let current_time = Clock::get()?.unix_timestamp;
        
        require!(escrow.state == EscrowState::AwaitingConfirmation, EscrowError::InvalidEscrowState);
        require!(current_time > (escrow.delivery_confirmed_at + 604800), EscrowError::AutoReleaseTimeNotReached);

        let transfer_instruction = Transfer{
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.retailer_token_account.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info()
        };
        
        let escrow = &mut ctx.accounts.escrow;
        
        let seeds= &[
            b"escrow".as_ref(),
            &escrow.escrow_id.to_le_bytes(),
            &[ctx.bumps.escrow]
        ]; 
        
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                transfer_instruction,
                &[seeds]
            ),
            escrow.amount
        )?;

        escrow.state = EscrowState::Completed;

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
        space = 8 + 32 + 32 + 8 + 8 + 1+ 8,
        seeds = [b"escrow", escrow_id.to_le_bytes().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this accoun
    pub retailer: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>
}

#[derive(Accounts)]
pub struct ConfirmDelivery<'info> {
    #[account(mut)]
    pub escrow: Account<'info, Escrow>,
    pub retailer: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(escrow_id:u64)]
pub struct ConfirmReceipt<'info> {
    #[account(
        mut,
        seeds = [b"escrow", &escrow_id.to_le_bytes()],
        bump)]
    pub escrow: Account<'info, Escrow>,
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub retailer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
#[instruction(escrow_id:u64)]
pub struct AutoRelease<'info> {
    #[account(mut, seeds= [b"escrow", &escrow_id.to_le_bytes()], bump)]
    pub escrow: Account<'info, Escrow>,
    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub retailer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}

#[account]
pub struct Escrow {
    pub buyer: Pubkey,
    pub retailer: Pubkey,
    pub escrow_id: u64,
    pub amount: u64,
    pub state: EscrowState, 
    pub delivery_confirmed_at: i64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum EscrowState { // usually enum's options take up 1 byte of space
    AwaitingDelivery,//0 // this will be set by the retailer
    AwaitingConfirmation, // this will be set by the buyer
    Completed
}

#[error_code]
pub enum EscrowError {
    #[msg("Invalid escrow state please check")]
    InvalidEscrowState,
    #[msg("Sorry, the confirmation period has expired!")]
    ConfirmationPeriodExpired,
    #[msg("Autorelease time not reached")]
    AutoReleaseTimeNotReached,
}


