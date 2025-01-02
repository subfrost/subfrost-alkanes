use crate::alkanes::dxbtc::{DxBtc, AlkaneId};
use alkanes_support::context::Context;
use alkanes_support::parcel::AlkaneTransferParcel;
use anyhow::Result;
use wasm_bindgen_test::wasm_bindgen_test;

fn setup_token() -> DxBtc {
    println!("Setting up new DxBtc token...");
    let token = DxBtc::default();
    let context = Context {
        myself: AlkaneId::new(1, 1),
        inputs: vec![0, 1, 2],
        incoming_alkanes: AlkaneTransferParcel::default(),
        caller: AlkaneId::new(1, 1),
        vout: 0,
    };
    println!("Created context with caller ID: {:?}", context.caller);
    DxBtc::set_mock_context(context);
    token
}

fn get_test_address() -> Vec<u8> {
    vec![1, 2, 3, 4, 5]
}

#[wasm_bindgen_test]
fn test_initialization() -> Result<()> {
    println!("\n=== Running initialization test ===");
    let token = setup_token();
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    {
        let deposit_token = token.deposit_token.lock().unwrap();
        assert!(deposit_token.is_some());
        println!("Deposit token verification successful");
    }
    Ok(())
}

#[wasm_bindgen_test]
fn test_initial_deposit() -> Result<()> {
    println!("\n=== Running initial deposit test ===");
    let token = setup_token();
    let deposit_amount: u64 = 1000;
    let sender = get_test_address();
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    println!("Making initial deposit of {} from sender {:?}", deposit_amount, sender);
    let transfer = token.deposit(deposit_amount, sender.clone())?;
    println!("Transfer completed: {:?}", transfer);
    
    let shares = token.get_shares(&sender);
    println!("Current shares: {}", shares);
    assert_eq!(shares, deposit_amount);
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply: {}", *total_supply);
        assert_eq!(*total_supply, deposit_amount);
    }
    
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, deposit_amount);
    }
    
    assert_eq!(transfer.value, deposit_amount as u128);
    Ok(())
}

#[wasm_bindgen_test]
fn test_multiple_deposits() -> Result<()> {
    println!("\n=== Running multiple deposits test ===");
    let token = setup_token();
    let sender = get_test_address();
    let first_deposit = 1000;
    let second_deposit = 500;
    let expected_mint = first_deposit + second_deposit;
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    println!("Making first deposit of {}", first_deposit);
    token.deposit(first_deposit, sender.clone())?;
    
    println!("Making second deposit of {}", second_deposit);
    let transfer = token.deposit(second_deposit, sender.clone())?;
    println!("Second transfer completed: {:?}", transfer);
    
    let shares = token.get_shares(&sender);
    println!("Final shares: {}", shares);
    assert_eq!(shares, expected_mint);
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply: {}", *total_supply);
        assert_eq!(*total_supply, expected_mint);
    }
    
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, expected_mint);
    }
    
    assert_eq!(transfer.value, second_deposit as u128);
    Ok(())
}

#[test]
fn test_withdrawal() -> Result<()> {
    println!("\n=== Running withdrawal test ===");
    let token = setup_token();
    let sender = get_test_address();
    let initial_deposit = 1000;
    let withdraw_shares = 500;
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    println!("Making initial deposit of {}", initial_deposit);
    token.deposit(initial_deposit, sender.clone())?;
    
    println!("Attempting withdrawal of {} shares", withdraw_shares);
    let (shares_transfer, deposit_transfer) = token.withdraw(withdraw_shares, sender.clone())?;
    println!("Withdrawal completed - Shares transfer: {:?}, Deposit transfer: {:?}", 
             shares_transfer, deposit_transfer);
    
    let shares = token.get_shares(&sender);
    println!("Remaining shares: {}", shares);
    assert_eq!(shares, initial_deposit - withdraw_shares);
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply: {}", *total_supply);
        assert_eq!(*total_supply, initial_deposit - withdraw_shares);
    }
    
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, initial_deposit - withdraw_shares);
    }
    
    assert_eq!(shares_transfer.value, withdraw_shares as u128);
    assert_eq!(deposit_transfer.value, withdraw_shares as u128);
    Ok(())
}

#[wasm_bindgen_test]
fn test_insufficient_shares_withdrawal() -> Result<()> {
    println!("\n=== Running insufficient shares withdrawal test ===");
    let token = setup_token();
    let sender = get_test_address();
    let initial_deposit = 1000;
    let withdraw_shares = 1500; // More than deposited
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    println!("Making initial deposit of {}", initial_deposit);
    token.deposit(initial_deposit, sender.clone())?;
    
    println!("Attempting withdrawal of {} shares (should fail)", withdraw_shares);
    let result = token.withdraw(withdraw_shares, sender.clone());
    println!("Withdrawal result: {:?}", result);
    assert!(result.is_err());
    Ok(())
}

#[wasm_bindgen_test]
fn test_multiple_users() -> Result<()> {
    println!("\n=== Running multiple users test ===");
    let token = setup_token();
    let user1 = vec![1, 2, 3];
    let user2 = vec![4, 5, 6];
    let deposit1 = 1000;
    let deposit2 = 500;
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    println!("Making deposit for user1: amount={}, address={:?}", deposit1, user1);
    token.deposit(deposit1, user1.clone())?;
    
    println!("Making deposit for user2: amount={}, address={:?}", deposit2, user2);
    let transfer = token.deposit(deposit2, user2.clone())?;
    println!("User2 transfer completed: {:?}", transfer);
    
    let user1_shares = token.get_shares(&user1);
    let user2_shares = token.get_shares(&user2);
    println!("Final shares - User1: {}, User2: {}", user1_shares, user2_shares);
    assert_eq!(user1_shares, deposit1);
    assert_eq!(user2_shares, deposit2);
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply: {}", *total_supply);
        assert_eq!(*total_supply, deposit1 + deposit2);
    }
    
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, deposit1 + deposit2);
    }
    
    assert_eq!(transfer.value, deposit2 as u128);
    Ok(())
}

// Additional tests for core functionality
#[wasm_bindgen_test]
fn test_core_functionality() -> Result<()> {
    println!("\n=== Running core functionality test ===");
    let token = setup_token();
    
    println!("Checking initial state...");
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Initial total supply: {}", *total_supply);
        assert_eq!(*total_supply, 0);
    }
    
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Initial total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, 0);
    }
    
    {
        let deposit_token = token.deposit_token.lock().unwrap();
        println!("Initial deposit token: {:?}", deposit_token);
        assert!(deposit_token.is_none());
    }
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    {
        let deposit_token = token.deposit_token.lock().unwrap();
        assert!(deposit_token.is_some());
        println!("Final deposit token state verified");
    }
    
    Ok(())
}

// Additional tests for payment functionality
#[wasm_bindgen_test]
fn test_payment_functionality() -> Result<()> {
    println!("\n=== Running payment functionality test ===");
    let token = setup_token();
    let sender = get_test_address();
    
    println!("Initializing deposit token...");
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(AlkaneId::new(1, 2));
        println!("Deposit token initialized: {:?}", deposit_token);
    }
    
    // Test small payment
    let small_amount = 100;
    println!("Testing small payment: amount={}", small_amount);
    let transfer = token.deposit(small_amount, sender.clone())?;
    println!("Small payment transfer completed: {:?}", transfer);
    assert_eq!(transfer.value, small_amount as u128);
    
    // Test large payment
    let large_amount = 1_000_000;
    println!("Testing large payment: amount={}", large_amount);
    let transfer = token.deposit(large_amount, sender.clone())?;
    println!("Large payment transfer completed: {:?}", transfer);
    assert_eq!(transfer.value, large_amount as u128);
    
    Ok(())
} 