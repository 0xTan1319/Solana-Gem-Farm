use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use gem_bank::program::GemBank;
use gem_bank::{self, cpi::accounts::InitBank, state::Bank};

use crate::state::*;

#[derive(Accounts)]
#[instruction(bump_auth: u8, bump_pot_a: u8, bump_pot_b: u8)]
pub struct InitFarm<'info> {
    // farm
    #[account(init, payer = payer, space = 8 + std::mem::size_of::<Farm>())]
    pub farm: Account<'info, Farm>,
    pub farm_manager: Signer<'info>,
    #[account(mut, seeds = [farm.key().as_ref()], bump = bump_auth)]
    pub farm_authority: AccountInfo<'info>,

    // todo need ixs to be able to update mints/pots
    // reward a
    #[account(init, seeds = [
            b"reward_pot".as_ref(),
            farm.key().as_ref(),
            reward_a_mint.key().as_ref(),
        ],
        bump = bump_pot_a,
        token::mint = reward_a_mint,
        token::authority = farm_authority,
        payer = payer)]
    pub reward_a_pot: Box<Account<'info, TokenAccount>>,
    pub reward_a_mint: Box<Account<'info, Mint>>,

    // reward b
    #[account(init, seeds = [
            b"reward_pot".as_ref(),
            farm.key().as_ref(),
            reward_b_mint.key().as_ref(),
        ],
        bump = bump_pot_b,
        token::mint = reward_b_mint,
        token::authority = farm_authority,
        payer = payer)]
    pub reward_b_pot: Box<Account<'info, TokenAccount>>,
    pub reward_b_mint: Box<Account<'info, Mint>>,

    // cpi
    // todo should it be less opinionated and simply take in a pre-made bank?
    //  current thinking no: coz we NEED the bank to be managed by the farm authority
    #[account(mut)]
    pub bank: Signer<'info>,
    pub gem_bank: Program<'info, GemBank>,

    // misc
    #[account(mut)]
    pub payer: Signer<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitFarm<'info> {
    fn init_bank_ctx(&self) -> CpiContext<'_, '_, '_, 'info, InitBank<'info>> {
        CpiContext::new(
            self.gem_bank.to_account_info(),
            InitBank {
                bank: self.bank.to_account_info(),
                // using farm_authority not farm_manager, coz latter can be re-assigned
                bank_manager: self.farm_authority.clone(),
                payer: self.payer.to_account_info(),
                system_program: self.system_program.to_account_info(),
            },
        )
    }
}

pub fn handler(ctx: Context<InitFarm>, bump_auth: u8) -> ProgramResult {
    //record new farm details
    let farm_key = ctx.accounts.farm.key().clone();
    let farm = &mut ctx.accounts.farm;

    farm.version = LATEST_FARM_VERSION;
    farm.farm_manager = ctx.accounts.farm_manager.key();
    farm.farm_authority = ctx.accounts.farm_authority.key();
    farm.farm_authority_seed = farm_key;
    farm.farm_authority_bump_seed = [bump_auth];
    farm.bank = ctx.accounts.bank.key();

    farm.reward_a.reward_mint = ctx.accounts.reward_a_mint.key();
    farm.reward_a.reward_pot = ctx.accounts.reward_a_pot.key();

    farm.reward_b.reward_mint = ctx.accounts.reward_b_mint.key();
    farm.reward_b.reward_pot = ctx.accounts.reward_b_pot.key();

    // todo worth manually init'ing all the variables at 0s?

    //do a cpi call to start a new bank
    gem_bank::cpi::init_bank(
        ctx.accounts
            .init_bank_ctx()
            .with_signer(&[&ctx.accounts.farm.farm_seeds()]),
    )?;

    msg!("new farm initialized");
    Ok(())
}
