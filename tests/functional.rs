
// use synchronizer::processor::Processor;
// use solana_program_test::*;
// use solana_sdk::{
//     account::Account,
//     instruction::{AccountMeta, Instruction},
//     pubkey::Pubkey,
//     signature::Signer,
//     transaction::Transaction,
// };

// use std::mem;

// // Functional tests
// #[tokio::test]
// async fn test_synchronizer() {
//     let synchronizer_key = Pubkey::new_unique(); // Payer??

//     let program_id = Pubkey::new_unique(); // TODO: not new
//     let mut program_test = ProgramTest::new(
//         "synchronizer",
//         program_id,
//         processor!(processor.process_instruction), // Run the native version with `cargo test`
//     );

//     program_test.add_account(
//         app_pubkey,
//         Account {
//             lamports: 5,
//             data: vec![0_u8; mem::size_of::<f32>()],
//             owner: program_id,
//             ..Account::default()
//         },
//     );
//     let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
// }
