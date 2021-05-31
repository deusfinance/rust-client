use solana_program::{hash::Hash, program_pack::Pack, rent::Rent, system_instruction};
use synchronizer::{processor::Processor, processor::id, state::SynchronizerData};
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError};

fn init_acc_minimum_balance() -> u64 {
    Rent::default().minimum_balance(SynchronizerData::get_packed_len())
}

pub async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint_account: &Keypair,
    mint_rent: u64,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint_account.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &owner,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, mint_account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), TransportError> {
    let rent = banks_client.get_rent().await.unwrap();
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &account.pubkey(),
                account_rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                owner,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

pub async fn mint_tokens_to(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            destination,
            &authority.pubkey(),
            &[&authority.pubkey()],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

// Functional tests
#[tokio::test]
async fn test_synchronizer() {
    let program_test = ProgramTest::new(
        "synchronizer",
        id(),
        processor!(Processor::process_instruction),
    );

    let (mut banks_client, synchronizer_key, recent_blockhash) = program_test.start().await;

    // TODO: spl_token infrastructure transactions
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
    let synchronizer_rent = rent.minimum_balance(SynchronizerData::LEN);

    let user_key = Keypair::new();

    // TODO: collateral token is not owned by synchronizer
    let collateral_token_key = Keypair::new();
    create_mint(
        &mut banks_client,
        &synchronizer_key,
        &recent_blockhash,
        &collateral_token_key,
        mint_rent,
        &synchronizer_key.pubkey()
    ).await.unwrap();

    let fiat_token_key = Keypair::new();
    create_mint(
        &mut banks_client,
        &synchronizer_key,
        &recent_blockhash,
        &fiat_token_key,
        mint_rent,
        &synchronizer_key.pubkey()
    ).await.unwrap();

    // sync associated accs
    // create_token_account(
    //     &mut banks_client,
    //     &synchronizer_key,
    //     &recent_blockhash,
    //     account,
    //     mint,
    //     owner
    // ).await.unwrap();
    // user associated accs

    // mint some tokens

    // TODO: sunchronizer transactions tests (all create_accounts by transactions)
    // init sync acc
    // do_process
}
