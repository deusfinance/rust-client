
use solana_program::{program_pack::Pack, rent::Rent};
use synchronizer::{processor::Processor, processor::id, state::SynchronizerData};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
};

fn init_acc_minimum_balance() -> u64 {
    Rent::default().minimum_balance(SynchronizerData::get_packed_len())
}

// Functional tests
#[tokio::test]
async fn test_synchronizer() {
    let program_test = ProgramTest::new(
        "synchronizer",
        id(),
        processor!(Processor::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;

    // TODO: spl_token infrastructure transactions
    // TODO: sunchronizer transactions tests (all create_accounts by transactions)
}
