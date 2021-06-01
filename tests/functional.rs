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

async fn sell_for(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>,
    fiat_mint: &Pubkey,
    user_collateral_token_account: &Pubkey,
    user_fiat_token_account: &Pubkey,
    synchronizer_collateral_token_account: &Pubkey,
    user_authority: &Keypair,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::sell_for(
                &id(),
                multiplier,
                amount,
                fee,
                &prices,
                &oracles,
                fiat_mint,
                user_collateral_token_account,
                user_fiat_token_account,
                synchronizer_collateral_token_account,
                &user_authority.pubkey(),
                &synchronizer_authority.pubkey()
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, user_authority, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn buy_for(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    multiplier: u64,
    amount: u64,
    fee: u64,
    prices: &Vec<u64>,
    oracles: &Vec<Pubkey>,
    fiat_mint: &Pubkey,
    user_collateral_token_account: &Pubkey,
    user_fiat_token_account: &Pubkey,
    synchronizer_collateral_token_account: &Pubkey,
    user_authority: &Keypair,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::buy_for(
                &id(),
                multiplier,
                amount,
                fee,
                &prices,
                &oracles,
                fiat_mint,
                user_collateral_token_account,
                user_fiat_token_account,
                synchronizer_collateral_token_account,
                &user_authority.pubkey(),
                &synchronizer_authority.pubkey()
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, user_authority, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn get_token_balance(banks_client: &mut BanksClient, token_account: &Pubkey) -> u64 {
    let account = banks_client.get_account(*token_account).await.unwrap().unwrap();
    let account_data= spl_token::state::Account::unpack_from_slice(account.data.as_slice()).unwrap();
    account_data.amount
}

async fn get_synchronizer_data(banks_client: &mut BanksClient, synchronizer_key: &Pubkey) -> SynchronizerData {
    let synch_acc = banks_client.get_account(*synchronizer_key).await.unwrap().unwrap();
    assert_eq!(id(), synch_acc.owner);
    synchronizer::state::SynchronizerData::unpack(&synch_acc.data).unwrap()
}

// Functional tests
#[tokio::test]
async fn test_synchronizer_public_api() {
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
        &payer,
        &recent_blockhash,
        &user_collateral_account,
        account_rent,
        &collateral_token_key.pubkey(),
        &user_key.pubkey()
    ).await.unwrap();

    let user_fiat_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &user_fiat_account,
        account_rent,
        &fiat_token_key.pubkey(),
        &user_key.pubkey()
    ).await.unwrap();

    // Mint some collateral tokens to Synchronizer
    mint_tokens_to(
        &mut banks_client,
        &payer,
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
        &payer,
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

    let oracles = vec![
        Keypair::new(),
        Keypair::new(),
    ];
    let oracle_pubkeys: Vec<Pubkey> = oracles.iter().map(|k| k.pubkey()).collect();

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

    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.is_initialized, true);
    assert_eq!(synchronizer.collateral_token_key, collateral_token_key.pubkey());
    assert_eq!(synchronizer.minimum_required_signature, 2);
    assert_eq!(synchronizer.remaining_dollar_cap, 500_000_000_000);
    assert_eq!(synchronizer.withdrawable_fee_amount, 0);

    let user_fiat_balance_before = get_token_balance(&mut banks_client, &user_fiat_account.pubkey()).await;
    let sync_collateral_balance_before = get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await;
    let user_collateral_balance_before = get_token_balance(&mut banks_client, &user_collateral_account.pubkey()).await;

    let mul_stocks = 2;
    let fee = spl_token::ui_amount_to_amount(0.001, decimals);
    let prices = vec![
        spl_token::ui_amount_to_amount(0.5, decimals),
        spl_token::ui_amount_to_amount(0.4, decimals)
    ];

    // Test buy_for instruction
    let buy_fiat_amount = spl_token::ui_amount_to_amount(200.0, decimals);
    buy_for(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        mul_stocks,
        buy_fiat_amount,
        fee,
        &prices,
        &oracle_pubkeys,
        &fiat_token_key.pubkey(),
        &user_collateral_account.pubkey(),
        &user_fiat_account.pubkey(),
        &synchronizer_collateral_account.pubkey(),
        &user_key,
        &synchronizer_key
    ).await.unwrap();

    // Check balances after buy_for
    assert_eq!(
        get_token_balance(&mut banks_client, &user_fiat_account.pubkey()).await,
        user_fiat_balance_before + buy_fiat_amount
    );

    let collateral_amount: u64 = 100_000_000_000; // amount * price
    let collateral_fee: u64 = 100_000_000; // collateral_amount * fee
    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        sync_collateral_balance_before + (collateral_amount + collateral_fee)
    );
    assert_eq!(
        get_token_balance(&mut banks_client, &user_collateral_account.pubkey()).await,
        user_collateral_balance_before - (collateral_amount + collateral_fee)
    );

    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.remaining_dollar_cap, 500_000_000_000 - (collateral_amount * mul_stocks));
    assert_eq!(synchronizer.withdrawable_fee_amount, 0 + collateral_fee);

    // TODO: check bad access

    let user_fiat_balance_before = get_token_balance(&mut banks_client, &user_fiat_account.pubkey()).await;
    let sync_collateral_balance_before = get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await;
    let user_collateral_balance_before = get_token_balance(&mut banks_client, &user_collateral_account.pubkey()).await;

    // Test sell_for instruction
    let sell_fiat_amount = spl_token::ui_amount_to_amount(100.0, decimals);
    sell_for(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        mul_stocks,
        sell_fiat_amount,
        fee,
        &prices,
        &oracle_pubkeys,
        &fiat_token_key.pubkey(),
        &user_collateral_account.pubkey(),
        &user_fiat_account.pubkey(),
        &synchronizer_collateral_account.pubkey(),
        &user_key,
        &synchronizer_key
    ).await.unwrap();

    // Check balances afet sell_for
    assert_eq!(
        get_token_balance(&mut banks_client, &user_fiat_account.pubkey()).await,
        user_fiat_balance_before - sell_fiat_amount
    );

    let collateral_amount: u64 = 40_000_000_000; // amount * price
    let collateral_fee: u64 = 40_000_000; // collateral_amount * fee
    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        sync_collateral_balance_before - (collateral_amount - collateral_fee)
    );
    assert_eq!(
        get_token_balance(&mut banks_client, &user_collateral_account.pubkey()).await,
        user_collateral_balance_before + (collateral_amount - collateral_fee)
    );

    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.remaining_dollar_cap, 300_000_000_000 + (collateral_amount * mul_stocks));
    assert_eq!(synchronizer.withdrawable_fee_amount, 100_000_000 + collateral_fee);

    // TODO: check bad access
}

// TODO: func test_withdraw
// TODO: func test admin setters (accses)
