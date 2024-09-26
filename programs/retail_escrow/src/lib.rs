use anchor_lang::prelude::*;

declare_id!("BjNAQd5f4YCY963A1CNscxGLTKFToSeyCoSXxh6QjzgS");

#[program]
pub mod retail_escrow {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
