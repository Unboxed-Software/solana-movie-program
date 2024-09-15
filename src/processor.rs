use crate::error::ReviewError;
use crate::instruction::MovieInstruction;
use crate::state::{MovieAccountState, MovieComment, MovieCommentCounter};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MovieInstruction::unpack(instruction_data)?;
    match instruction {
        MovieInstruction::AddMovieReview {
            title,
            rating,
            description,
        } => add_movie_review(program_id, accounts, title, rating, description),
        MovieInstruction::UpdateMovieReview {
            title,
            rating,
            description,
        } => update_movie_review(program_id, accounts, title, rating, description),
        MovieInstruction::AddComment { comment } => add_comment(program_id, accounts, comment),
    }
}

pub fn add_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    msg!(
        "Adding movie review... Title: {}, Rating: {}",
        title,
        rating
    );

    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Validate signer
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive and validate PDA
    let (pda, bump_seed) = Pubkey::find_program_address(
        &[initializer.key.as_ref(), title.as_bytes().as_ref()],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    // Validate rating
    if rating > 5 || rating < 1 {
        msg!("Rating cannot be higher than 5");
        return Err(ReviewError::InvalidRating.into());
    }

    // Validate data length
    let account_len: usize = 1000;
    let total_len: usize = MovieAccountState::get_account_size(title.clone(), description.clone());
    if total_len > account_len {
        return Err(ReviewError::InvalidDataLength.into());
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);
    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_account.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&[
            initializer.key.as_ref(),
            title.as_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;

    msg!("PDA created: {}", pda);

    // Deserialize and initialize account data
    msg!("Unpacking state account");
    let mut account_data = MovieAccountState::try_from_slice(&pda_account.data.borrow())?;
    msg!("Borrowed account data");

    msg!("Checking if movie account is already initialized");
    if account_data.is_initialized() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Set account data
    account_data.discriminator = MovieAccountState::DISCRIMINATOR.to_string();
    account_data.reviewer = *initializer.key;
    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    // Serialize account data
    msg!("Serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("State account serialized");

    create_comment_counter(program_id, accounts, pda)?;

    Ok(())
}

pub fn update_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    msg!("Updating movie review...");

    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    // Validate account ownership and signer
    if pda_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Deserialize account data
    msg!("Unpacking state account");
    let mut account_data = MovieAccountState::try_from_slice(&pda_account.data.borrow())?;
    msg!("Review title: {}", account_data.title);

    // Validate PDA
    let (pda, _bump_seed) = Pubkey::find_program_address(
        &[
            initializer.key.as_ref(),
            account_data.title.as_bytes().as_ref(),
        ],
        program_id,
    );
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    // Check account initialization
    msg!("Checking if movie account is initialized");
    if !account_data.is_initialized() {
        msg!("Account is not initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    // Validate rating
    if rating > 5 || rating < 1 {
        msg!("Invalid Rating");
        return Err(ReviewError::InvalidRating.into());
    }

    // Check data length
    let update_len = MovieAccountState::get_account_size(title, description.clone());
    if update_len > 1000 {
        msg!("Data length is larger than 1000 bytes");
        return Err(ReviewError::InvalidDataLength.into());
    }

    // Log review details before update
    msg!("Review before update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    // Update account data
    account_data.rating = rating;
    account_data.description = description;

    // Log review details after update
    msg!("Review after update:");
    msg!("Title: {}", account_data.title);
    msg!("Rating: {}", account_data.rating);
    msg!("Description: {}", account_data.description);

    // Serialize updated account data
    msg!("Serializing account");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("State account serialized");

    Ok(())
}

pub fn add_comment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    comment: String,
) -> ProgramResult {
    msg!("Adding Comment: {}", comment);

    let account_info_iter = &mut accounts.iter();
    let commenter = next_account_info(account_info_iter)?;
    let pda_review = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let pda_comment = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let mut counter_data = MovieCommentCounter::try_from_slice(&pda_counter.data.borrow())?;
    let account_len = MovieComment::get_account_size(comment.clone());

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );
    if pda != *pda_comment.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    invoke_signed(
        &system_instruction::create_account(
            commenter.key,
            pda_comment.key,
            rent_lamports,
            account_len.try_into().unwrap(),
            program_id,
        ),
        &[
            commenter.clone(),
            pda_comment.clone(),
            system_program.clone(),
        ],
        &[&[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;
    msg!("Created Comment Account");

    let mut comment_data = MovieComment::try_from_slice(&pda_comment.data.borrow())?;
    if comment_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    comment_data.discriminator = MovieComment::DISCRIMINATOR.to_string();
    comment_data.review = *pda_review.key;
    comment_data.commenter = *commenter.key;
    comment_data.comment = comment;
    comment_data.is_initialized = true;
    comment_data.serialize(&mut &mut pda_comment.data.borrow_mut()[..])?;

    msg!("Comment Count: {}", counter_data.counter);
    counter_data.counter += 1;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    Ok(())
}

fn create_comment_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    pda: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let rent = Rent::get()?;
    let counter_rent_lamports = rent.minimum_balance(MovieCommentCounter::SIZE);

    let (counter, counter_bump) =
        Pubkey::find_program_address(&[pda.as_ref(), "comment".as_ref()], program_id);
    if counter != *pda_counter.key {
        msg!("Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            pda_counter.key,
            counter_rent_lamports,
            MovieCommentCounter::SIZE.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_counter.clone(),
            system_program.clone(),
        ],
        &[&[pda.as_ref(), "comment".as_ref(), &[counter_bump]]],
    )?;
    msg!("Comment counter created");

    let mut counter_data = MovieCommentCounter::try_from_slice(&pda_counter.data.borrow())?;
    if counter_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    counter_data.discriminator = MovieCommentCounter::DISCRIMINATOR.to_string();
    counter_data.counter = 0;
    counter_data.is_initialized = true;
    counter_data.serialize(&mut &mut pda_counter.data.borrow_mut()[..])?;

    Ok(())
}
