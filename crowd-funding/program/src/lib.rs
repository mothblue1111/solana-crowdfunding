// First we include what we are going to need in our program. 
// This  is the Rust style of importing things.
// Remember we added the dependencies in cargo.toml
// And from the `solana_program` crate we are including  all the required things.
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use borsh::{BorshDeserialize, BorshSerialize};


// Every solana program has one entry point
// And it is a convention to name it `process_instruction`. 
// It should take in program_id, accounts, instruction_data as parameters.
fn process_instruction(
    // program id is nothing but the id of this program on the solana network.
    program_id: &Pubkey,
    // When we invoke our program we can 
    // give meta data of all the account we 
    // want to work with.
    // As you can see it is a array of AccountInfo.
    // We can provide as many as we want.
    accounts: &[AccountInfo],
    // This is the data we want to process our instruction for.
    // It is a list of 8 bitunsigned integers(0..255).
    instruction_data: &[u8],
    
    // Here we specify the return type.
    // If you know a little bit of typescript. 
    // This was of writing types and returns types might we familiar to you.
) -> ProgramResult {
    
    if instruction_data.len() == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if instruction_data[0] == 0 {
        return create_campaign(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()]
        );
    } else if instruction_data[0] == 1 {
        return withdraw(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()]
        );
    } else if instruction_data[0] == 2{
        return donate(
            program_id,
            accounts,
            &instruction_data[1..instruction_data.len()]
        );
    }
    
    msg!("Can not find the entrypoint required!");
    Err(ProgramError::InvalidInstructionData)
}

fn create_campaign(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    
    /// Writing account or we can call it program account.
    /// This is an account we will create in our front-end.
    /// This account should br owned by the solana program.
    let writing_account = next_account_info(accounts_iter)?;

    /// Account of the person creating the campaign.
    let creator_account = next_account_info(accounts_iter)?;

    // Now to allow transactions we want the creator account to sign the transaction.
    if !creator_account.is_signer {
        msg!("creator_account should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }
    /// We want to write in this account so we want its owner by the program.
    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut input_data = CampaignDetails::try_from_slice(&instruction_data)
    .expect("Instruction data serialization didn't worked");


    if input_data.admin != *creator_account.key {
        msg!("Invaild instruction data");
        return Err(ProgramError::InvalidInstructionData);
    }

    // get the minimum balance we need in our program account.
    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());
    /// And we make sure our program account (`writing_account`) has that much lamports(balance).
    if **writing_account.lamports.borrow() < rent_exemption {
        msg!("The balance of writing_account should be more then rent_exemption");
        return Err(ProgramError::InsufficientFunds);
    }
    // Then we can set the initial amount donate to be zero.
    input_data.amount_donated=0;

    input_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}

fn withdraw(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    let admin_account = next_account_info(accounts_iter)?;
    
    // We check if the writing account is owned by program.
    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    // Admin account should be the signer in this trasaction.
    if !admin_account.is_signer {
        msg!("admin should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }
    let campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
    .expect("Error deserializing data");

    if campaign_data.admin != *admin_account.key {
        msg!("Only the account admin can withdraw");
        return Err(ProgramError::InvalidAccountData);
    }

    let input_data = WithdrawRequest::try_from_slice(&instruction_data)
        .expect("Instruction data serialization didn't worked");

    let rent_exemption = Rent::get()?.minimum_balance(writing_account.data_len());

    if **writing_account.lamports.borrow() - rent_exemption < input_data.amount {
        msg!("Insufficent balance");
        return Err(ProgramError::InsufficientFunds);
    }


    **writing_account.try_borrow_mut_lamports()? -= input_data.amount;
    **admin_account.try_borrow_mut_lamports()? += input_data.amount;

    Ok(())
}

fn donate(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let writing_account = next_account_info(accounts_iter)?;
    let donator_program_account = next_account_info(accounts_iter)?;
    let donator = next_account_info(accounts_iter)?;

    if writing_account.owner != program_id {
        msg!("writing_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if donator_program_account.owner != program_id {
        msg!("donator_program_account isn't owned by program");
        return Err(ProgramError::IncorrectProgramId);
    }
    if !donator.is_signer {
        msg!("donator should be signer");
        return Err(ProgramError::IncorrectProgramId);
    }
    let mut campaign_data = CampaignDetails::try_from_slice(*writing_account.data.borrow())
        .expect("Error deserializing data");

    campaign_data.amount_donated += **donator_program_account.lamports.borrow();
    **writing_account.try_borrow_mut_lamports()? += **donator_program_account.lamports.borrow();
    **donator_program_account.try_borrow_mut_lamports()? = 0;

    campaign_data.serialize(&mut &mut writing_account.data.borrow_mut()[..])?;

    Ok(())
}

// Then we call the entry point macro to add `process_instruction` as our entry point to our program.
entrypoint!(process_instruction);


#[derive( BorshSerialize, BorshDeserialize, Debug)]
struct CampaignDetails {
    pub admin: Pubkey,
    pub name: String,
    pub description: String,
    pub image_lin: String,
    pub amount_donated: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct WithdrawRequest {
    pub amount: u64,
}