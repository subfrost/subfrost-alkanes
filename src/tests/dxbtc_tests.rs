use crate::alkanes::dxbtc::{DxBtc, AlkaneId};
use alkanes_support::context::Context;
use alkanes_support::parcel::AlkaneTransferParcel;
use anyhow::Result;
use wasm_bindgen_test::*;

#[allow(unused_imports)]
use {
    metashrew::{println, stdio::stdout},
    std::fmt::Write,
};


// Helper function to print detailed state
fn print_token_state(token: &DxBtc, label: &str) {
    println!("\n=== {} ===", label);
    
    // Print deposit token state
    {
        let deposit_token = token.deposit_token.lock().unwrap();
        println!("Deposit Token: {:?}", *deposit_token);
    }
    
    // Print total supply
    {
        let supply = token.total_supply.lock().unwrap();
        println!("Total Supply: {}", *supply);
    }
    
    // Print total deposits
    {
        let deposits = token.total_deposits.lock().unwrap();
        println!("Total Deposits: {}", *deposits);
    }
    
    // Print balances if any exist
    {
        let balances = token.balances.lock().unwrap();
        println!("Current Balances: {:?}", *balances);
    }
    
    println!("=== End {} ===\n", label);
}

fn setup_token() -> DxBtc {
    println!("\n>>> Setting up new DxBtc token...");
    let token = DxBtc::default();
    
    let context = Context {
        myself: AlkaneId::new(1, 1),
        inputs: vec![0, 1, 2],
        incoming_alkanes: AlkaneTransferParcel::default(),
        caller: AlkaneId::new(1, 1),
        vout: 0,
    };
    
    println!("Created context:");
    println!("  Caller ID: {:?}", context.caller);
    println!("  Self ID: {:?}", context.myself);
    println!("  Inputs: {:?}", context.inputs);
    println!("  Vout: {}", context.vout);
    
    DxBtc::set_mock_context(context);
    print_token_state(&token, "Initial Token State");
    token
}

fn get_test_address() -> Vec<u8> {
    let addr = vec![1, 2, 3, 4, 5];
    println!("Generated test address: {:02x?}", addr);
    addr
}

#[wasm_bindgen_test]
fn test_initialization() -> Result<()> {
    println!("\n====== Running Initialization Test ======");
    let token = setup_token();
    
    println!("\n>>> Initializing deposit token...");
    let deposit_token_id = AlkaneId::new(1, 2);
    println!("Using deposit token ID: {:?}", deposit_token_id);
    
    {
        println!("Attempting to acquire deposit_token lock...");
        let mut deposit_token = token.deposit_token.lock().unwrap();
        println!("Lock acquired successfully");
        *deposit_token = Some(deposit_token_id);
        println!("Deposit token set to: {:?}", *deposit_token);
    }
    println!("Lock released");
    
    // Verify in a separate block
    {
        println!("\n>>> Verifying initialization...");
        println!("Attempting to acquire deposit_token lock for verification...");
        let deposit_token = token.deposit_token.lock().unwrap();
        assert!(deposit_token.is_some());
        println!("Verification successful - deposit token is set");
        println!("Current value: {:?}", *deposit_token);
    }
    
    print_token_state(&token, "Final Token State");
    println!("====== Initialization Test Complete ======\n");
    Ok(())
}

#[wasm_bindgen_test]
fn test_core_functionality() -> Result<()> {
    println!("\n====== Running Core Functionality Test ======");
    let token = setup_token();
    
    println!("\n>>> Checking initial state...");
    // Check initial values with detailed logging
    let initial_supply = {
        println!("Acquiring total_supply lock...");
        let supply = token.total_supply.lock().unwrap();
        println!("Lock acquired successfully");
        let value = *supply;
        println!("Current supply: {}", value);
        value
    };
    assert_eq!(initial_supply, 0, "Initial supply should be 0");
    
    let initial_deposits = {
        println!("Acquiring total_deposits lock...");
        let deposits = token.total_deposits.lock().unwrap();
        println!("Lock acquired successfully");
        let value = *deposits;
        println!("Current deposits: {}", value);
        value
    };
    assert_eq!(initial_deposits, 0, "Initial deposits should be 0");
    
    let initial_token = {
        println!("Acquiring deposit_token lock...");
        let token = token.deposit_token.lock().unwrap();
        println!("Lock acquired successfully");
        let value = token.clone();
        println!("Current deposit token: {:?}", value);
        value
    };
    assert!(initial_token.is_none(), "Initial deposit token should be None");
    
    println!("\n>>> Initializing deposit token...");
    let deposit_token_id = AlkaneId::new(1, 2);
    println!("Created deposit token ID: {:?}", deposit_token_id);
    
    {
        println!("Attempting to acquire deposit_token lock for initialization...");
        let mut deposit_token = token.deposit_token.lock().unwrap();
        println!("Lock acquired successfully");
        *deposit_token = Some(deposit_token_id);
        println!("Deposit token initialized: {:?}", *deposit_token);
    }
    println!("Lock released");
    
    // Verify final state
    println!("\n>>> Verifying final state...");
    let final_token = {
        println!("Acquiring deposit_token lock for final verification...");
        let token = token.deposit_token.lock().unwrap();
        println!("Lock acquired successfully");
        let value = token.clone();
        println!("Final deposit token value: {:?}", value);
        value
    };
    assert!(final_token.is_some(), "Final deposit token should be Some");
    
    print_token_state(&token, "Final Token State");
    println!("====== Core Functionality Test Complete ======\n");
    Ok(())
}

#[wasm_bindgen_test]
fn test_deposit_functionality() -> Result<()> {
    println!("\n====== Running Deposit Functionality Test ======");
    let token = setup_token();
    let test_addr = get_test_address();
    
    // Initialize deposit token first
    println!("\n>>> Setting up deposit token...");
    let deposit_token_id = AlkaneId::new(1, 2);
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(deposit_token_id);
        println!("Deposit token initialized: {:?}", *deposit_token);
    }
    
    // Make initial deposit
    let initial_deposit = 1000;
    println!("\n>>> Making initial deposit of {} tokens", initial_deposit);
    println!("Address: {:?}", test_addr);
    
    let transfer = token.deposit(initial_deposit, test_addr.clone())?;
    println!("Transfer result: {:?}", transfer);
    
    // Verify deposit state
    println!("\n>>> Verifying deposit state...");
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits after: {}", *total_deposits);
        assert_eq!(*total_deposits, initial_deposit, "Total deposits should match initial deposit");
    }
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply after: {}", *total_supply);
        assert_eq!(*total_supply, initial_deposit, "Total supply should match initial deposit");
    }
    
    let shares = token.get_shares(&test_addr);
    println!("Shares for address: {}", shares);
    assert_eq!(shares, initial_deposit, "Shares should match deposit amount");
    
    print_token_state(&token, "Post Initial Deposit State");
    
    // Make second deposit
    let second_deposit = 500;
    println!("\n>>> Making second deposit of {} tokens", second_deposit);
    let transfer = token.deposit(second_deposit, test_addr.clone())?;
    println!("Transfer result: {:?}", transfer);
    
    // Verify cumulative state
    println!("\n>>> Verifying cumulative state...");
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Total deposits after: {}", *total_deposits);
        assert_eq!(*total_deposits, initial_deposit + second_deposit, "Total deposits should be cumulative");
    }
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Total supply after: {}", *total_supply);
        assert_eq!(*total_supply, initial_deposit + second_deposit, "Total supply should be cumulative");
    }
    
    let shares = token.get_shares(&test_addr);
    println!("Final shares for address: {}", shares);
    assert_eq!(shares, initial_deposit + second_deposit, "Final shares should be cumulative");
    
    print_token_state(&token, "Final State After Second Deposit");
    
    print_token_state(&token, "Final State After Second Deposit");
    println!("====== Deposit Functionality Test Complete ======\n");
    Ok(())
}

#[wasm_bindgen_test]
fn test_multiple_users_deposit() -> Result<()> {
    println!("\n====== Running Multiple Users Deposit Test ======");
    let token = setup_token();
    
    // Initialize deposit token
    println!("\n>>> Setting up deposit token...");
    let deposit_token_id = AlkaneId::new(1, 2);
    {
        let mut deposit_token = token.deposit_token.lock().unwrap();
        *deposit_token = Some(deposit_token_id);
        println!("Deposit token initialized: {:?}", *deposit_token);
    }
    
    // Create two test addresses
    let addr1 = vec![1, 2, 3, 4, 5];
    let addr2 = vec![6, 7, 8, 9, 10];
    println!("Created test addresses:");
    println!("Address 1: {:?}", addr1);
    println!("Address 2: {:?}", addr2);
    
    // First user deposit
    let deposit1 = 1000;
    println!("\n>>> User 1 depositing {} tokens", deposit1);
    let transfer = token.deposit(deposit1, addr1.clone())?;
    println!("Transfer result: {:?}", transfer);
    
    print_token_state(&token, "After First User Deposit");
    
    // Second user deposit
    let deposit2 = 500;
    println!("\n>>> User 2 depositing {} tokens", deposit2);
    let transfer = token.deposit(deposit2, addr2.clone())?;
    println!("Transfer result: {:?}", transfer);
    
    // Verify individual balances
    let shares1 = token.get_shares(&addr1);
    let shares2 = token.get_shares(&addr2);
    println!("\n>>> Final share balances:");
    println!("User 1 shares: {}", shares1);
    println!("User 2 shares: {}", shares2);
    
    assert_eq!(shares1, deposit1, "User 1 shares should match their deposit");
    assert_eq!(shares2, deposit2, "User 2 shares should match their deposit");
    
    // Verify total state
    {
        let total_deposits = token.total_deposits.lock().unwrap();
        println!("Final total deposits: {}", *total_deposits);
        assert_eq!(*total_deposits, deposit1 + deposit2, "Total deposits should be sum of both deposits");
    }
    
    {
        let total_supply = token.total_supply.lock().unwrap();
        println!("Final total supply: {}", *total_supply);
        assert_eq!(*total_supply, deposit1 + deposit2, "Total supply should be sum of both deposits");
    }
    
    print_token_state(&token, "Final State After Both Users");
    println!("====== Multiple Users Deposit Test Complete ======\n");
    Ok(())
}

// Let's start with just these two tests to verify the approach works 