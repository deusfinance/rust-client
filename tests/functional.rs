use borsh::de::BorshDeserialize;
use byteorder::{BigEndian, WriteBytesExt};
use synchronizer::processor::{process_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

use std::convert::TryInto;
use std::mem;

// Functional tests
#[tokio::test]
async fn test_smartcontract_template() {
    let program_id = Pubkey::new_unique();
    let app_pubkey = Pubkey::new_unique();

    let mut program_test = ProgramTest::new(
        "template_contract", // Run the BPF version with `cargo test-bpf`
        program_id,
        processor!(process_instruction), // Run the native version with `cargo test`
    );

    program_test.add_account(
        app_pubkey,
        Account {
            lamports: 5,
            data: vec![0_u8; mem::size_of::<f32>()],
            owner: program_id,
            ..Account::default()
        },
    );
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
}
