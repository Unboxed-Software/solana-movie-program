#[cfg(test)]
mod tests {
    use {
        crate::processor::*,
        assert_matches::*,
        solana_program::{
            instruction::{AccountMeta, Instruction},
            system_program::ID as SYSTEM_PROGRAM_ID,
        },
        solana_program_test::*,
        solana_sdk::{
            signature::Signer,
            transaction::Transaction,
            sysvar::rent::ID as SYSVAR_RENT_ID
        },
        spl_associated_token_account::{
            get_associated_token_address,
            instruction::create_associated_token_account,
        },
        spl_token:: ID as TOKEN_PROGRAM_ID,
        //crate::processor::{initialize_token_mint, add_movie_review},
    };
 
    // First unit test
    #[tokio::test]
    async fn test_add_movie_review_instruction() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "pda_local",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;
 
        // Derive PDA for token mint authority
        let (mint, _bump_seed) = Pubkey::find_program_address(&[b"token_mint"], &program_id);
        let (mint_auth, _bump_seed) = Pubkey::find_program_address(&[b"token_auth"], &program_id);

        let init_mint_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new_readonly(payer.pubkey(), true),
                AccountMeta::new(mint, false),
                AccountMeta::new(mint_auth, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(SYSVAR_RENT_ID, false)
            ],
            data: vec![3]
        };

        // Create review PDA
        let title: String = "Captain America".to_owned();
        const RATING: u8 = 3;
        let review: String = "Liked the movie".to_owned();
        let (review_pda, _bump_seed) =
            Pubkey::find_program_address(&[payer.pubkey().as_ref(), title.as_bytes()], &program_id);
        
        // Create comment PDA
        let (comment_pda, _bump_seed) =
            Pubkey::find_program_address(&[review_pda.as_ref(), b"comment"], &program_id);
        
        // Create user associate token account of token mint
        let init_ata_ix: Instruction = create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
        );
        
        let user_ata: Pubkey =
            get_associated_token_address(&payer.pubkey(), &mint);

        // Concat data to single buffer
        let mut data_vec = vec![0];
        data_vec.append(
            &mut (TryInto::<u32>::try_into(title.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data_vec.append(&mut title.into_bytes());
        data_vec.push(RATING);
        data_vec.append(
            &mut (TryInto::<u32>::try_into(review.len())
                .unwrap()
                .to_le_bytes())
            .try_into()
            .unwrap(),
        );
        data_vec.append(&mut review.into_bytes());

        // Create transaction object with instructions, accounts, and input data
        let mut transaction = Transaction::new_with_payer(
            &[
            init_mint_ix,
            init_ata_ix,
            Instruction {
                program_id: program_id,
                accounts: vec![
                    AccountMeta::new_readonly(payer.pubkey(), true),
                    AccountMeta::new(review_pda, false),
                    AccountMeta::new(comment_pda, false),
                    AccountMeta::new(mint, false),
                    AccountMeta::new_readonly(mint_auth, false),
                    AccountMeta::new(user_ata, false),
                    AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                    AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                ],
                data: data_vec,
            },
            ],
            Some(&payer.pubkey()),
        );
        transaction.sign(&[&payer], recent_blockhash);

        // Process transaction and compare the result
        assert_matches!(banks_client.process_transaction(transaction).await, Ok(_));
    }
}