use solana_program::{hash::Hash, instruction::InstructionError, program_pack::Pack, system_instruction};
use synchronizer::{processor::Processor, processor::id, state::SynchronizerData};
use solana_program_test::*;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::{Signer, SignerError}, transaction::{Transaction, TransactionError}, transport::TransportError};

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

async fn withdraw_fee(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    amount: u64,
    synchronizer_collateral_token_account: &Pubkey,
    recipient_collateral_token_account: &Pubkey,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::withdraw_fee(
                &id(),
                amount,
                &synchronizer_collateral_token_account,
                &recipient_collateral_token_account,
                &synchronizer_authority.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn withdraw_collateral(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    amount: u64,
    synchronizer_collateral_token_account: &Pubkey,
    recipient_collateral_token_account: &Pubkey,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::withdraw_collateral(
                &id(),
                amount,
                &synchronizer_collateral_token_account,
                &recipient_collateral_token_account,
                &synchronizer_authority.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn set_collateral_token(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    collateral_token_key: &Pubkey,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::set_collateral_token(
                &id(),
                collateral_token_key,
                &synchronizer_authority.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn set_remaining_dollar_cap(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    remaining_dollar_cap: u64,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::set_remaining_dollar_cap(
                &id(),
                remaining_dollar_cap,
                &synchronizer_authority.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_authority], *recent_blockhash);
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

async fn set_minimum_required_signature(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    recent_blockhash: &Hash,
    minimum_required_signature: u64,
    synchronizer_authority: &Keypair,
) -> Result<(), TransportError> {
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::set_minimum_required_signature(
                &id(),
                minimum_required_signature,
                &synchronizer_authority.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[payer, synchronizer_authority], *recent_blockhash);
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

    // Case: too big amount
    let mut amount = get_token_balance(&mut banks_client, &user_collateral_account.pubkey()).await;
    amount += spl_token::ui_amount_to_amount(500.0, decimals);
    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(3)),
        buy_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            amount,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    let mut amount = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await.remaining_dollar_cap;
    amount += spl_token::ui_amount_to_amount(500.0, decimals);
    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(3)),
        sell_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            amount,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    // Case: change collateral token key
    let new_collateral_token_key = Keypair::new();
    set_collateral_token(&mut banks_client, &payer, &recent_blockhash, &new_collateral_token_key.pubkey(), &synchronizer_key).await.unwrap();

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(8)),
        buy_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            123_000_000_000,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(8)),
        sell_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            123_000_000_000,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    // Case: Change minimum required signatures
    set_collateral_token(&mut banks_client, &payer, &recent_blockhash, &collateral_token_key.pubkey(), &synchronizer_key).await.unwrap();
    set_minimum_required_signature(&mut banks_client, &payer, &recent_blockhash, 5, &synchronizer_key).await.unwrap();

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(5)),
        buy_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            124_000_000_000,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(5)),
        sell_for(
            &mut banks_client,
            &payer,
            &recent_blockhash,
            2,
            124_000_000_000,
            1_000_000,
            &prices,
            &oracle_pubkeys,
            &fiat_token_key.pubkey(),
            &user_collateral_account.pubkey(),
            &user_fiat_account.pubkey(),
            &synchronizer_collateral_account.pubkey(),
            &user_key,
            &synchronizer_key
        ).await.unwrap_err().unwrap(),
    );

    set_minimum_required_signature(&mut banks_client, &payer, &recent_blockhash, 2, &synchronizer_key).await.unwrap();

    // Case: bad user fiat account ownership
    let fake_user_key = Keypair::new();
    let fake_user_fiat_acc = Keypair::new();
    create_token_account(&mut banks_client, &payer, &recent_blockhash,
        &fake_user_fiat_acc,
        account_rent,
        &fiat_token_key.pubkey(),
        &fake_user_key.pubkey()
    ).await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::sell_for(
                &id(),
                2,
                50_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &user_collateral_account.pubkey(),
                &fake_user_fiat_acc.pubkey(), // bad acc
                &synchronizer_collateral_account.pubkey(),
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::buy_for(
                &id(),
                2,
                50_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &user_collateral_account.pubkey(),
                &fake_user_fiat_acc.pubkey(), // bad acc
                &synchronizer_collateral_account.pubkey(),
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );

    // Case: bad collateral account
    let fake_user_collateral_acc = Keypair::new();
    create_token_account(&mut banks_client, &payer, &recent_blockhash,
        &fake_user_collateral_acc,
        account_rent,
        &collateral_token_key.pubkey(),
        &fake_user_key.pubkey()
    ).await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::sell_for(
                &id(),
                2,
                51_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &fake_user_collateral_acc.pubkey(), // bad acc
                &user_fiat_account.pubkey(),
                &synchronizer_collateral_account.pubkey(),
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::buy_for(
                &id(),
                2,
                51_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &fake_user_collateral_acc.pubkey(), // bad acc
                &user_fiat_account.pubkey(),
                &synchronizer_collateral_account.pubkey(),
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );

    // Case: bad synchronizer collateral account ownership
    let fake_synch_key = Keypair::new();
    let fake_synch_collateral_acc = Keypair::new();
    create_token_account(&mut banks_client, &payer, &recent_blockhash,
        &fake_synch_collateral_acc,
        account_rent,
        &collateral_token_key.pubkey(),
        &fake_synch_key.pubkey()
    ).await.unwrap();

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::sell_for(
                &id(),
                2,
                51_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &user_collateral_account.pubkey(),
                &user_fiat_account.pubkey(),
                &fake_synch_collateral_acc.pubkey(), // // bad acc
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );

    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::buy_for(
                &id(),
                2,
                51_000_000_000,
                1_000_000,
                &prices,
                &oracle_pubkeys,
                &fiat_token_key.pubkey(),
                &user_collateral_account.pubkey(),
                &user_fiat_account.pubkey(),
                &fake_synch_collateral_acc.pubkey(), // bad acc
                &user_key.pubkey(),
                &synchronizer_key.pubkey()
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &user_key, &synchronizer_key], recent_blockhash);

    assert_eq!(
        TransactionError::InstructionError(0, InstructionError::Custom(4)),
        banks_client.process_transaction(transaction).await.unwrap_err().unwrap()
    );
}

#[tokio::test]
async fn test_synchronizer_admin_setters() {
    let program_test = ProgramTest::new(
        "synchronizer",
        id(),
        processor!(Processor::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let synchronizer_key = Keypair::new();
    let collateral_token_key = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let synchronizer_data_rent = rent.minimum_balance(SynchronizerData::LEN);

    // Initialize Synchronizer account
    let decimals = 9;
    let remaining_dollar_cap = spl_token::ui_amount_to_amount(500.0, decimals);
    let withdrawable_fee_amount = 0;
    let oracles = vec![
        Keypair::new(),
        Keypair::new(),
    ];

    initialize_synchronizer_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        synchronizer_data_rent,
        &collateral_token_key.pubkey(),
        remaining_dollar_cap,
        withdrawable_fee_amount,
        oracles.len() as u64,
        &synchronizer_key
    ).await.unwrap();

    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.is_initialized, true);
    assert_eq!(synchronizer.collateral_token_key, collateral_token_key.pubkey());
    assert_eq!(synchronizer.remaining_dollar_cap, 500_000_000_000);
    assert_eq!(synchronizer.withdrawable_fee_amount, 0);
    assert_eq!(synchronizer.minimum_required_signature, 2);

    set_remaining_dollar_cap(&mut banks_client, &payer, &recent_blockhash, 123500_000_000_000, &synchronizer_key).await.unwrap();
    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.collateral_token_key, collateral_token_key.pubkey());
    assert_eq!(synchronizer.remaining_dollar_cap, 123500_000_000_000);
    assert_eq!(synchronizer.minimum_required_signature, 2);

    set_minimum_required_signature(&mut banks_client, &payer, &recent_blockhash, 123, &synchronizer_key).await.unwrap();
    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.collateral_token_key, collateral_token_key.pubkey());
    assert_eq!(synchronizer.remaining_dollar_cap, 123500_000_000_000);
    assert_eq!(synchronizer.minimum_required_signature, 123);

    let new_token_key = Pubkey::new_unique();
    set_collateral_token(&mut banks_client, &payer, &recent_blockhash, &new_token_key, &synchronizer_key).await.unwrap();
    let synchronizer = get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await;
    assert_eq!(synchronizer.collateral_token_key, new_token_key);
    assert_eq!(synchronizer.remaining_dollar_cap, 123500_000_000_000);
    assert_eq!(synchronizer.minimum_required_signature, 123);

    // BadCase: bad account owner
    let badowner_synchronizer_key = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &badowner_synchronizer_key.pubkey(),
                synchronizer_data_rent,
                synchronizer::state::SynchronizerData::LEN as u64,
                &spl_token::id(), // bad account owner
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &badowner_synchronizer_key], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    assert_eq!(
        set_remaining_dollar_cap(&mut banks_client, &payer, &recent_blockhash, 250_000_000_000, &badowner_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(4))
    );
    assert_eq!(
        set_collateral_token(&mut banks_client, &payer, &recent_blockhash, &Pubkey::new_unique(), &badowner_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(4))
    );
    assert_eq!(
        set_minimum_required_signature(&mut banks_client, &payer, &recent_blockhash, 123, &badowner_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(4))
    );

    // BadCase: account not initialized
    let fake_synchronizer_key = Keypair::new();
    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &payer.pubkey(),
                &fake_synchronizer_key.pubkey(),
                synchronizer_data_rent,
                synchronizer::state::SynchronizerData::LEN as u64,
                &id(),
            ),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &fake_synchronizer_key], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    assert_eq!(
        set_remaining_dollar_cap(&mut banks_client, &payer, &recent_blockhash, 250_000_000_000, &fake_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(1))
    );
    assert_eq!(
        set_collateral_token(&mut banks_client, &payer, &recent_blockhash, &Pubkey::new_unique(), &fake_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(1))
    );
    assert_eq!(
        set_minimum_required_signature(&mut banks_client, &payer, &recent_blockhash, 123, &fake_synchronizer_key).await.unwrap_err().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(1))
    );

    // BadCase: bad signer
    let mut transaction = Transaction::new_with_payer(
        &[
            synchronizer::instruction::initialize_synchronizer_account(
                &id(),
                &collateral_token_key.pubkey(),
                remaining_dollar_cap,
                withdrawable_fee_amount,
                oracles.len() as u64,
                &fake_synchronizer_key.pubkey(),
            )
            .unwrap(),
        ],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer, &fake_synchronizer_key], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Trying to update other synchronizer account
    let mut transaction = Transaction::new_with_payer(
        &[synchronizer::instruction::set_minimum_required_signature(
                &id(),
                123,
                &synchronizer_key.pubkey(),
            )
            .unwrap()
        ],
        Some(&payer.pubkey()),
    );

    assert_eq!(
        transaction.try_sign(&[&payer, &fake_synchronizer_key], recent_blockhash),
        Err(SignerError::KeypairPubkeyMismatch)
    )
}

#[tokio::test]
async fn test_synchronizer_withdraw() {
    let program_test = ProgramTest::new(
        "synchronizer",
        id(),
        processor!(Processor::process_instruction),
    );

    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    let synchronizer_key = Keypair::new();
    let recipient_key = Keypair::new();
    let collateral_owner_key = Keypair::new();

    let rent = banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);
    let account_rent = rent.minimum_balance(spl_token::state::Account::LEN);
    let synchronizer_data_rent = rent.minimum_balance(SynchronizerData::LEN);

    // Infrastructure preparing
    // Create collateral token mint
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

    // Create and init token associated accounts for synchronizer
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

    // Create token associated accounts for recipient
    let recipient_collateral_account = Keypair::new();
    create_token_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &recipient_collateral_account,
        account_rent,
        &collateral_token_key.pubkey(),
        &recipient_key.pubkey()
    ).await.unwrap();

    // Mint some collateral asset to synchronizer account
    mint_tokens_to(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &collateral_token_key.pubkey(),
        &synchronizer_collateral_account.pubkey(),
        &collateral_owner_key,
        spl_token::ui_amount_to_amount(500.0, decimals)
    ).await.unwrap();

    // Mint some collateral asset to recipient account
    mint_tokens_to(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        &collateral_token_key.pubkey(),
        &recipient_collateral_account.pubkey(),
        &collateral_owner_key,
        spl_token::ui_amount_to_amount(100.0, decimals)
    ).await.unwrap();

    // Initialize Synchronizer account
    let oracles = vec![
        Keypair::new(),
        Keypair::new(),
    ];

    initialize_synchronizer_account(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        synchronizer_data_rent,
        &collateral_token_key.pubkey(),
        spl_token::ui_amount_to_amount(500.0, decimals),
        spl_token::ui_amount_to_amount(250.0, decimals),
        oracles.len() as u64,
        &synchronizer_key
    ).await.unwrap();

    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        500_000_000_000
    );
    assert_eq!(
        get_token_balance(&mut banks_client, &recipient_collateral_account.pubkey()).await,
        100_000_000_000
    );
    assert_eq!(
        get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await.withdrawable_fee_amount,
        250_000_000_000
    );

    // Test withdraw_fee
    let amount = spl_token::ui_amount_to_amount(50.0, decimals);
    withdraw_fee(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        amount,
        &synchronizer_collateral_account.pubkey(),
        &recipient_collateral_account.pubkey(),
        &synchronizer_key
    ).await.unwrap();

    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        450_000_000_000
    );
    assert_eq!(
        get_token_balance(&mut banks_client, &recipient_collateral_account.pubkey()).await,
        150_000_000_000
    );
    assert_eq!(
        get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await.withdrawable_fee_amount,
        200_000_000_000
    );

    // Test withdraw_collateral
    let amount = spl_token::ui_amount_to_amount(50.0, decimals);
    // Processor::process_withdraw_collateral(&accounts, amount).unwrap();
    withdraw_collateral(
        &mut banks_client,
        &payer,
        &recent_blockhash,
        amount,
        &synchronizer_collateral_account.pubkey(),
        &recipient_collateral_account.pubkey(),
        &synchronizer_key
    ).await.unwrap();

    assert_eq!(
        get_token_balance(&mut banks_client, &synchronizer_collateral_account.pubkey()).await,
        400_000_000_000
    );
    assert_eq!(
        get_token_balance(&mut banks_client, &recipient_collateral_account.pubkey()).await,
        200_000_000_000
    );
    assert_eq!(
        get_synchronizer_data(&mut banks_client, &synchronizer_key.pubkey()).await.withdrawable_fee_amount,
        200_000_000_000
    );
}
