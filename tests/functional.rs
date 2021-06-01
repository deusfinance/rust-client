use solana_program::{hash::Hash, program_pack::Pack, system_instruction};
use synchronizer::{processor::Processor, processor::id, state::SynchronizerData};
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction, transport::TransportError};

async fn create_mint(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    mint_account: &Keypair,
    mint_rent: u64,
    owner: &Pubkey,
    decimals: u8,
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
                decimals,
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, mint_account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn create_token_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    account: &Keypair,
    account_rent: u64,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Result<(), TransportError> {
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

async fn mint_tokens_to(
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
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn initialize_synchronizer_account(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    synchronizer_data_rent: u64,
    collateral_token_key: &Pubkey,
    remaining_dollar_cap: u64,
    withdrawable_fee_amount: u64,
    minimum_required_signature: u64,
    synchronizer_account: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &synchronizer_account.pubkey(),
                synchronizer_data_rent,
                synchronizer::state::SynchronizerData::LEN as u64,
                &id(),
            ),
            synchronizer::instruction::initialize_synchronizer_account(
                &id(),
                collateral_token_key,
                remaining_dollar_cap,
                withdrawable_fee_amount,
                minimum_required_signature,
                &synchronizer_account.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_account], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn get_token_balance(banks_client: &mut BanksClient, token_account: &Pubkey) -> u64 {
    let account = banks_client.get_account(*token_account).await.unwrap().unwrap();
    let account_data= spl_token::state::Account::unpack_from_slice(account.data.as_slice()).unwrap();
    account_data.amount
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
    let synchronizer_key = Keypair::new();
    let user_key = Keypair::new();
    let collateral_owner_key = Keypair::new();

    // Prepare tokens infrastructure
    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
    let synchronizer_data_rent = rent.minimum_balance(SynchronizerData::LEN);

    // Create tokens mint
    let decimals = Processor::DEFAULT_DECIMALS;
    let collateral_token_key = Keypair::new();
    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &collateral_token_key,
        mint_rent,
        &collateral_owner_key.pubkey(),
        decimals
    ).await.unwrap();

    let fiat_token_key = Keypair::new();
    create_mint(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &fiat_token_key,
        mint_rent,
        &synchronizer_key.pubkey(),
        decimals
    ).await.unwrap();

    // Create synchronizer associated accounts
    let synchronizer_collateral_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &synchronizer_collateral_account,
        account_rent,
        &collateral_token_key.pubkey(),
        &synchronizer_key.pubkey()
    ).await.unwrap();

    let synchronizer_fiat_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &synchronizer_fiat_account,
        account_rent,
        &fiat_token_key.pubkey(),
        &synchronizer_key.pubkey()
    ).await.unwrap();

    // Create user associated accounts
    let user_collateral_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer, // TODO: user must pay?
        &recent_blockhash,
        &user_collateral_account,
        account_rent,
        &collateral_token_key.pubkey(),
        &user_key.pubkey()
    ).await.unwrap();

    let user_fiat_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer, // TODO: user must pay?
        &recent_blockhash,
        &user_fiat_account,
        account_rent,
        &fiat_token_key.pubkey(),
        &user_key.pubkey()
    ).await.unwrap();

    // Mint some collateral tokens to Synchronizer
    mint_tokens_to(
        &mut banks_client,
        &payer, // TODO: who is payer?
        &recent_blockhash,
        &collateral_token_key.pubkey(),
        &synchronizer_collateral_account.pubkey(),
        &collateral_owner_key,
        spl_token::ui_amount_to_amount(500.0, decimals)
    ).await.unwrap();

    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        500_000_000_000
    );

    // Mint some collateral tokens to user
    mint_tokens_to(
        &mut banks_client,
        &payer, // TODO: who is payer?
        &recent_blockhash,
        &collateral_token_key.pubkey(),
        &user_collateral_account.pubkey(),
        &collateral_owner_key,
        spl_token::ui_amount_to_amount(500.0, decimals)
    ).await.unwrap();

    let synchronizer_collateral_balance = get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await;
    assert_eq!(
        synchronizer_collateral_balance,
        500_000_000_000
    );

    // TODO: sunchronizer transactions tests (all create_accounts by transactions)
    let oracles = [
        Keypair::new(),
        Keypair::new(),
    ];

    // Initialize Synchronizer account
    initialize_synchronizer_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        synchronizer_data_rent,
        &collateral_token_key.pubkey(),
        synchronizer_collateral_balance,
        0,
        oracles.len() as u64,
        &synchronizer_key
    ).await.unwrap();

    let synch_acc = banks_client.get_account(synchronizer_key.pubkey()).await.unwrap().unwrap();
    assert_eq!(id(), synch_acc.owner);
    let synchronizer = synchronizer::state::SynchronizerData::unpack(&synch_acc.data).unwrap();
    assert_eq!(synchronizer.is_initialized, true);
    assert_eq!(synchronizer.collateral_token_key, collateral_token_key.pubkey());
    assert_eq!(synchronizer.minimum_required_signature, 2);
    assert_eq!(synchronizer.remaining_dollar_cap, 500_000_000_000);
    assert_eq!(synchronizer.withdrawable_fee_amount, 0);

    // Test sell_for instruction
    // Test buy_for instruction
    // TODO (check access): Test admin setter instruction
}
