use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Mint, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("GWumKLTcpvB6DkiqqEkQxkmUiYHX6Bpw8cR7YzTxjejD");

#[program]
pub mod escrow {
    use super::*;

    const ESCROW_PDA_SEED: &[u8] = b"escrow";

    //Initialize is what happens when the input accounts are assigned to the EscrowAccount fields one by one.
    // Then, a PDA is derived to be going to become new authority of the initializer_deposit_token_account. *******

    pub fn initialize(
        ctx: Context<Initialize>,
        _vault_account_bump: u8,
        initializer_amount: u64,
        taker_amount: u64,
    ) -> ProgramResult {
        ctx.accounts.escrow_account.initializer_key = *ctx.accounts.initializer.key;
        ctx.accounts
            .escrow_account
            .initializer_deposit_token_account = *ctx
            .accounts
            .initializer_deposit_token_account
            .to_account_info()
            .key;
        ctx.accounts
            .escrow_account
            .initializer_receive_token_account = *ctx
            .accounts
            .initializer_receive_token_account
            .to_account_info()
            .key;
        ctx.accounts.escrow_account.initializer_amount = initializer_amount;
        ctx.accounts.escrow_account.taker_amount = taker_amount;

        let (vault_authority, _vault_authority_bump) =
            Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        token::set_authority(
            ctx.accounts.into_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(vault_authority),
        )?;

        token::transfer(
            ctx.accounts.into_transfer_to_pda_context(),
            ctx.accounts.escrow_account.initializer_amount,
        )?;

        Ok(())
    }

    //Cancel simply resets the authority from PDA back to initializer

    pub fn cancel(ctx: Context<Cancel>) -> ProgramResult {
        let (_vault_authority, vault_authority_bump) =
            Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        let authority_seeds = &[&ESCROW_PDA_SEED[..], &[vault_authority_bump]];

        token::transfer(
            ctx.accounts
                .into_transfer_to_initializer_context()
                .with_signer(&[&authority_seeds[..]]),
            ctx.accounts.escrow_account.initializer_amount,
        )?;

        token::close_account(
            ctx.accounts
                .into_close_context()
                .with_signer(&[&authority_seeds[..]]),
        )?;

        Ok(())
    }

    //In exchange three things happen:
    // 1. First, Token A gets transfered from pda_deposit_token_account to taker_receive_token_account
    // 2. Next, Token B gets transfered from taker_deposit_token_account to initializer_receive_token_account
    // 3. Finally, the authority of pda_deposit_token_account gets set back to the initializer

    pub fn exchange(ctx: Context<Exchange>) -> ProgramResult {
       let (_vault_authority, vault_authority_bump) =
           Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
       let authority_seeds = &[&ESCROW_PDA_SEED[..], &[vault_authority_bump]];

       token::transfer(
           ctx.accounts.into_transfer_to_initializer_context(),
           ctx.accounts.escrow_account.taker_amount,
       )?;

       token::transfer(
           ctx.accounts
               .into_transfer_to_taker_context()
               .with_signer(&[&authority_seeds[..]]),
           ctx.accounts.escrow_account.initializer_amount,
       )?;

       token::close_account(
           ctx.accounts
               .into_close_context()
               .with_signer(&[&authority_seeds[..]]),
       )?;

       Ok(())
   }
}

//Instructions

//should be bringing in the the accounts needed for operations

//Constraint Attributes
// #[account(signer)] - Checks the given account signed the transaction
// #[account(mut)] - Marks the account as mutable and persists the state transition
// #[account(constraint = <expression\>)] - Executes the given code as a constraint, The expression shpuld evaluate to a boolean
// #[account(close = <target]>)] - Marks the account as being closed at the end of the instruction's execution, sending rent exemption lamports to the specified

//Initialize instruction info:
//pub initializer - Signer of InitialEscrow instruction, to be stored in EscrowAccount
//pub mint - The mint of exchange
//pub vault_account - The amount of Vault, which is created by Anchor via constraints
//pub initializer_deposit_token_account - The account of token acount for token exchange, to be stored in EscrowAccount
//pub initializer_receive_token_account- The account of token acount for token exchange, to be stored in EscrowAccount
//pub escrow_account - The account of EscrowAccount
//pub system_program - System Program
//pub rent - Rent
//pub token_program - The account of TokenProgram
#[derive(Accounts)]
#[instruction(vault_account_bump: u8, initializer_amount: u64)]
pub struct Initialize<'info> {
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"token-seed".as_ref()],
        bump = vault_account_bump,
        payer = initializer,
        token::mint = mint,
        token::authority = initializer,
    )]
    pub vault_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = initializer_deposit_token_account.amount >= initializer_amount
    )]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,
    pub initializer_receive_token_account: Account<'info, TokenAccount>,
    #[account(zero)]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,
    pub system_program: AccountInfo<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: AccountInfo<'info>,
}

//Cancel instruction info:
//pub initializer - initializer of EscrowAccount
//pub vault_account - The Program Derived Address
//pub vault_authority - The Program Derived Address
//pub initializer_deposit_token_account - The address of the token account for token exchange
//pub escrow_account - The address of EscrowAccount, have to check if the EscrowAccount follows certain constraints
//pub token_program - The address of TokenProgram
#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,
    pub vault_authority: AccountInfo<'info>,
    #[account(mut)]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = escrow_account.initializer_key == *initializer.key,
        constraint = escrow_account.initializer_deposit_token_account == *initializer_deposit_token_account.to_account_info().key,
        close = initializer
    )]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,
    pub token_program: AccountInfo<'info>,
}

//Exchange instruction info:
//pub taker - Signer of Exchange instruction
//pub taker_deposit_token_account - Token account of token exchange
//pub taker_receive_token_account- Token account of token exchange
//pub initializer_deposit_token_account - Token account of token exchange
//pub initializer_receive_token_account- Token account of token exchange
//pub initializer - To be used in constraints
//pub escrow_account - The address of EscrowAccount, have to check if the EscrowAccount follows certain constraints
//pub vault_account - The Program Derived Address
//pub vault_authority - The Program Derived Address
//pub token_program - The account of TokenProgram
#[derive(Accounts)]
pub struct Exchange<'info> {
    #[account(signer)]
    pub taker: AccountInfo<'info>,
    #[account(mut)]
    pub taker_deposit_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub taker_receive_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer_deposit_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer_receive_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initializer: AccountInfo<'info>,
    #[account(
        mut,
        constraint = escrow_account.taker_amount <= taker_deposit_token_account.amount,
        constraint = escrow_account.initializer_deposit_token_account == *initializer_deposit_token_account.to_account_info().key,
        constraint = escrow_account.initializer_receive_token_account == *initializer_receive_token_account.to_account_info().key,
        constraint = escrow_account.initializer_key == *initializer.key,
        close = initializer
    )]
    pub escrow_account: Box<Account<'info, EscrowAccount>>,
    #[account(mut)]
    pub vault_account: Account<'info, TokenAccount>,
    pub vault_authority: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
}

// You can see there are 2 different types for account: AccountInfo and Account.
// So what is the difference?
//   I suppose it’s proper to use Account over AccountInfo when you want Anchor to deserialize the data for convenience.
// In that case, you can access the account data via a trivial method call.
//   For example: ctx.accounts.vault_account.mint


// Accounts that are owned and managed by the program are defined in the #[account] section

//pub initializer_key - To authorize the actions properly
//pub initializer_deposit_token_account - to record the deposit account of initializer
//pub initializer_receive_token_account - to record the receiving account of the initialize
//pub initializer_amount - to record how much token should the initializer transfer to the taker
//pub taker_amount - to record how much token should the initializer recived from the taker


//Design an account that stores the minimum information to validate the escrow state and keep integrity of the program

#[account]
pub struct EscrowAccount {
    pub initializer_key: Pubkey,
    pub initializer_deposit_token_account: Pubkey,
    pub initializer_receive_token_account: Pubkey,
    pub initializer_amount: u64,
    pub taker_amount: u64,

}

// Utils for wrapping the data to be passed in token::transfer, tokem::close_account, token::set_authority.

impl<'info> Initialize<'info> {
    fn into_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .initializer_deposit_token_account
                .to_account_info()
                .clone(),
            to: self.vault_account.to_account_info().clone(),
            authority: self.initializer.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.vault_account.to_account_info().clone(),
            current_authority: self.initializer.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> Cancel<'info> {
    fn into_transfer_to_initializer_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self
                .initializer_deposit_token_account
                .to_account_info()
                .clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_account.to_account_info().clone(),
            destination: self.initializer.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> Exchange<'info> {
    fn into_transfer_to_initializer_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.taker_deposit_token_account.to_account_info().clone(),
            to: self
                .initializer_receive_token_account
                .to_account_info()
                .clone(),
            authority: self.taker.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_transfer_to_taker_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_account.to_account_info().clone(),
            to: self.taker_receive_token_account.to_account_info().clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn into_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.vault_account.to_account_info().clone(),
            destination: self.initializer.clone(),
            authority: self.vault_authority.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}
