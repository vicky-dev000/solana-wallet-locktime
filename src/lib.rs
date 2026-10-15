#![allow(unexpected_cfgs)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;

// Program ID of the deployed on-chain program.
pub const ID: Pubkey = Pubkey::new_from_array([
    6, 178, 60, 6, 1, 34, 206, 30, 34, 68, 45, 183, 214, 219, 31, 178, 140, 253, 133, 67, 111, 30,
    112, 146, 138, 243, 211, 116, 90, 209, 3, 41,
]);
pub const CONFIG_LEN: usize = 73;
pub const VERSION: u8 = 1;

const CONFIG_SEED: &[u8] = b"config";
const WALLET_SEED: &[u8] = b"wallet";
const LOCKTIME_NOT_REACHED: u32 = 6000;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub version: u8,
    pub instance_id: [u8; 32],
    pub authority: Pubkey,
    pub lock_until: i64,
}

impl Config {
    pub fn deserialize(data: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(data).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct CpiAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl CpiAccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool, is_writable: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
enum WalletInstruction {
    Initialize {
        instance_id: [u8; 32],
        lock_until: i64,
    },
    Execute {
        accounts: Vec<CpiAccountMeta>,
        data: Vec<u8>,
    },
}

pub fn config_pda(authority: &Pubkey, instance_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED, authority.as_ref(), instance_id], &ID)
}

pub fn wallet_pda(config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WALLET_SEED, config.as_ref()], &ID)
}

pub fn initialize_instruction(
    payer: Pubkey,
    authority: Pubkey,
    instance_id: [u8; 32],
    lock_until: i64,
) -> Instruction {
    let (config, _) = config_pda(&authority, &instance_id);
    Instruction {
        program_id: ID,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: borsh::to_vec(&WalletInstruction::Initialize {
            instance_id,
            lock_until,
        })
        .expect("instruction serialization"),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_instruction(
    authority: Pubkey,
    config: Pubkey,
    wallet: Pubkey,
    target_program: Pubkey,
    cpi_accounts: Vec<CpiAccountMeta>,
    data: Vec<u8>,
    remaining_accounts: Vec<AccountMeta>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new(wallet, false),
        AccountMeta::new_readonly(target_program, false),
    ];
    accounts.extend(remaining_accounts);
    Instruction {
        program_id: ID,
        accounts,
        data: borsh::to_vec(&WalletInstruction::Execute {
            accounts: cpi_accounts,
            data,
        })
        .expect("instruction serialization"),
    }
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Keep deployed identity, client address derivation and signer seeds aligned.
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    match WalletInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        WalletInstruction::Initialize {
            instance_id,
            lock_until,
        } => process_initialize(program_id, accounts, instance_id, lock_until),
        WalletInstruction::Execute {
            accounts: metas,
            data,
        } => process_execute(program_id, accounts, metas, data),
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instance_id: [u8; 32],
    lock_until: i64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;
    let config = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;

    if !payer.is_signer || !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if *system_program_account.key != system_program::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    if lock_until <= Clock::get()?.unix_timestamp {
        return Err(ProgramError::InvalidArgument);
    }

    let (expected_config, bump) = config_pda(authority.key, &instance_id);
    if expected_config != *config.key {
        return Err(ProgramError::InvalidSeeds);
    }

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(CONFIG_LEN);
    let signer_seeds: &[&[u8]] = &[CONFIG_SEED, authority.key.as_ref(), &instance_id, &[bump]];
    if config.lamports() == 0 {
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                config.key,
                required_lamports,
                CONFIG_LEN as u64,
                program_id,
            ),
            &[
                payer.clone(),
                config.clone(),
                system_program_account.clone(),
            ],
            &[signer_seeds],
        )?;
    } else {
        if config.owner != &system_program::ID || !config.data_is_empty() || config.executable {
            return Err(ProgramError::InvalidAccountData);
        }
        let top_up = required_lamports.saturating_sub(config.lamports());
        if top_up > 0 {
            solana_program::program::invoke(
                &system_instruction::transfer(payer.key, config.key, top_up),
                &[
                    payer.clone(),
                    config.clone(),
                    system_program_account.clone(),
                ],
            )?;
        }
        invoke_signed(
            &system_instruction::allocate(config.key, CONFIG_LEN as u64),
            &[config.clone(), system_program_account.clone()],
            &[signer_seeds],
        )?;
        invoke_signed(
            &system_instruction::assign(config.key, program_id),
            &[config.clone(), system_program_account.clone()],
            &[signer_seeds],
        )?;
    }

    Config {
        version: VERSION,
        instance_id,
        authority: *authority.key,
        lock_until,
    }
    .serialize(&mut &mut config.data.borrow_mut()[..])
    .map_err(|_| ProgramError::AccountDataTooSmall)
}

fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    metas: Vec<CpiAccountMeta>,
    data: Vec<u8>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let authority = next_account_info(accounts_iter)?;
    let config_account = next_account_info(accounts_iter)?;
    let wallet = next_account_info(accounts_iter)?;
    let target_program = next_account_info(accounts_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if config_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let config = Config::deserialize(&config_account.data.borrow())?;
    if config.version != VERSION || config.authority != *authority.key {
        return Err(ProgramError::InvalidArgument);
    }

    let (expected_config, _) = config_pda(&config.authority, &config.instance_id);
    if expected_config != *config_account.key {
        return Err(ProgramError::InvalidSeeds);
    }
    let (expected_wallet, wallet_bump) = wallet_pda(config_account.key);
    if expected_wallet != *wallet.key {
        return Err(ProgramError::InvalidSeeds);
    }
    if Clock::get()?.unix_timestamp < config.lock_until {
        return Err(ProgramError::Custom(LOCKTIME_NOT_REACHED));
    }
    if !target_program.executable {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut cpi_account_infos = Vec::with_capacity(metas.len() + 1);
    let mut cpi_metas = Vec::with_capacity(metas.len());
    for meta in metas {
        // Fixed accounts are also valid CPI inputs; do not require duplicates.
        let account = accounts
            .iter()
            .find(|account| *account.key == meta.pubkey)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        if meta.is_signer && meta.pubkey != *wallet.key && !account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if meta.is_writable && !account.is_writable {
            return Err(ProgramError::InvalidArgument);
        }
        cpi_metas.push(AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        });
        cpi_account_infos.push(account.clone());
    }
    cpi_account_infos.push(target_program.clone());

    let instruction = Instruction {
        program_id: *target_program.key,
        accounts: cpi_metas,
        data,
    };
    invoke_signed(
        &instruction,
        &cpi_account_infos,
        &[&[WALLET_SEED, config_account.key.as_ref(), &[wallet_bump]]],
    )
}
