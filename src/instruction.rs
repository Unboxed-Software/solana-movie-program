use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

pub enum MovieInstruction {
    AddMovieReview {
        title: String,
        rating: u8,
        description: String,
    },
    UpdateMovieReview {
        title: String,
        rating: u8,
        description: String,
    },
}

#[derive(BorshDeserialize)]
struct MovieReviewPayload {
    title: String,
    rating: u8,
    description: String,
}

impl MovieInstruction {
    pub const INSTRUCTION_DISCRIMINATOR_SIZE: usize = 1;

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Self::INSTRUCTION_DISCRIMINATOR_SIZE {
            return Err(ProgramError::InvalidInstructionData);
        }

        let (instruction_discriminator, rest) =
            input.split_at(Self::INSTRUCTION_DISCRIMINATOR_SIZE);
        let payload = MovieReviewPayload::try_from_slice(rest)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match instruction_discriminator[0] {
            0 => Ok(Self::AddMovieReview {
                title: payload.title,
                rating: payload.rating,
                description: payload.description,
            }),
            1 => Ok(Self::UpdateMovieReview {
                title: payload.title,
                rating: payload.rating,
                description: payload.description,
            }),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
