#[cfg(test)]
mod tests {
    use crate::alkanes::dxbtc::DxBtc;
    use alkanes_support::context::Context;
    use alkanes_support::id::AlkaneId;
    use alkanes_support::parcel::AlkaneTransferParcel;
    use anyhow::Result;
    use wasm_bindgen_test::*;

    #[allow(unused_imports)]
    use {
        metashrew::{println, stdio::stdout},
        std::fmt::Write,
    };

    fn setup_token() -> (DxBtc, Context) {
        println!("\n=== Setting up DxBtc Token ===");
        let token = DxBtc::default();
        let context = Context {
            myself: AlkaneId::new(1, 1),
            inputs: vec![],
            incoming_alkanes: AlkaneTransferParcel::default(),
            caller: AlkaneId::new(0, 0),
            vout: 0,
        };
        
        println!("Created context:");
        println!("  Self ID: {:?}", context.myself);
        println!("  Caller ID: {:?}", context.caller);
        println!("  Vout: {}", context.vout);
        
        // Initialize deposit token
        *token.deposit_token.borrow_mut() = Some(AlkaneId::new(1, 2));
        println!("Initialized deposit token: {:?}", token.deposit_token.borrow());
        
        DxBtc::set_mock_context(context.clone());
        println!("=== Token Setup Complete ===\n");
        (token, context)
    }

    fn print_token_state(token: &DxBtc, msg: &str) {
        let mut output = String::new();
        writeln!(output, "\n=== Token State: {} ===", msg).unwrap();
        writeln!(output, "Deposit Token: {:?}", token.deposit_token.borrow()).unwrap();
        writeln!(output, "Total Supply: {}", token.total_supply.borrow()).unwrap();
        writeln!(output, "Total Deposits: {}", token.total_deposits.borrow()).unwrap();
        writeln!(output, "==============================").unwrap();
        println!("{}", output);
    }

    fn print_user_balance(token: &DxBtc, user: &[u8], label: &str) {
        println!(">>> {} balance: {}", label, token.get_shares(user));
    }

    #[wasm_bindgen_test]
    fn test_initialization() -> Result<()> {
        println!("\n====== Running Initialization Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");
        
        // Verify initial state
        assert!(token.deposit_token.borrow().is_some(), "Deposit token should be initialized");
        assert_eq!(*token.total_supply.borrow(), 0, "Initial supply should be 0");
        assert_eq!(*token.total_deposits.borrow(), 0, "Initial deposits should be 0");
        
        println!("====== Initialization Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_deposit_functionality() -> Result<()> {
        println!("\n====== Running Deposit Functionality Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        let sender = vec![1, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Making initial deposit");
        println!("Amount: 1000");
        println!("Sender: {:02x?}", sender);
        
        // First deposit
        let transfer = token.deposit(1000, sender.clone())?;
        println!("First deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After First Deposit");
        
        // Verify first deposit
        assert_eq!(token.get_shares(&sender), 1000, "User should have 1000 shares after first deposit");
        assert_eq!(*token.total_supply.borrow(), 1000, "Total supply should be 1000");
        assert_eq!(*token.total_deposits.borrow(), 1000, "Total deposits should be 1000");
        
        println!("\n>>> Making second deposit");
        println!("Amount: 500");
        println!("Sender: {:02x?}", sender);
        
        // Second deposit
        let transfer = token.deposit(500, sender.clone())?;
        println!("Second deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After Second Deposit");
        
        // Verify cumulative state
        assert_eq!(token.get_shares(&sender), 1500, "User should have 1500 shares after second deposit");
        assert_eq!(*token.total_supply.borrow(), 1500, "Total supply should be 1500");
        assert_eq!(*token.total_deposits.borrow(), 1500, "Total deposits should be 1500");
        
        println!("====== Deposit Functionality Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_users_deposit() -> Result<()> {
        println!("\n====== Running Multiple Users Deposit Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let user2 = vec![2, 0, 0, 0, 0, 0, 0, 0];
        
        println!("\n>>> User 1 depositing");
        println!("Amount: 1000");
        println!("Address: {:02x?}", user1);
        
        let transfer = token.deposit(1000, user1.clone())?;
        println!("User 1 deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After User1 Deposit");
        
        println!("\n>>> User 2 depositing");
        println!("Amount: 500");
        println!("Address: {:02x?}", user2);
        
        let transfer = token.deposit(500, user2.clone())?;
        println!("User 2 deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After User2 Deposit");
        
        // Verify individual balances
        assert_eq!(token.get_shares(&user1), 1000, "User1 should have 1000 shares");
        assert_eq!(token.get_shares(&user2), 500, "User2 should have 500 shares");
        
        // Verify total state
        assert_eq!(*token.total_supply.borrow(), 1500, "Total supply should be 1500");
        assert_eq!(*token.total_deposits.borrow(), 1500, "Total deposits should be 1500");
        
        println!("====== Multiple Users Deposit Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_multiple_users_deposit_and_withdraw() -> Result<()> {
        println!("\n====== Running Multiple Users Deposit and Withdraw Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // Create 5 users
        let users: Vec<Vec<u8>> = (1..=5)
            .map(|i| vec![i as u8, 0, 0, 0, 0, 0, 0, 0])
            .collect();

        // Initial deposits
        println!("\n=== Initial Deposits ===");
        for (i, user) in users.iter().enumerate() {
            let amount = (i + 1) * 1000;
            println!("\n>>> User {} depositing", i + 1);
            println!("Amount: {}", amount);
            println!("Address: {:02x?}", user);
            
            let transfer = token.deposit(amount as u64, user.clone())?;
            println!("Deposit completed. Transfer result: {:?}", transfer);
            print_user_balance(&token, user, &format!("User {}", i + 1));
        }
        print_token_state(&token, "After All Deposits");

        // Partial withdrawals
        println!("\n=== Partial Withdrawals ===");
        for (i, user) in users.iter().enumerate() {
            let withdraw_amount = (i + 1) * 200;
            println!("\n>>> User {} withdrawing", i + 1);
            println!("Amount: {}", withdraw_amount);
            println!("Address: {:02x?}", user);
            
            let transfers = token.withdraw(withdraw_amount as u64, user.clone())?;
            println!("Withdrawal completed. Transfer results: {:?}", transfers);
            print_user_balance(&token, user, &format!("User {}", i + 1));
        }
        print_token_state(&token, "After Partial Withdrawals");

        // Full withdrawals for users 1, 3, and 5
        println!("\n=== Full Withdrawals (Users 1, 3, 5) ===");
        for i in [0, 2, 4] {
            let user = &users[i];
            let remaining_balance = token.get_shares(user);
            println!("\n>>> User {} withdrawing remaining balance", i + 1);
            println!("Amount: {}", remaining_balance);
            println!("Address: {:02x?}", user);
            
            let transfers = token.withdraw(remaining_balance, user.clone())?;
            println!("Withdrawal completed. Transfer results: {:?}", transfers);
            print_user_balance(&token, user, &format!("User {}", i + 1));
        }
        print_token_state(&token, "After Full Withdrawals");

        // Verify final state
        println!("\n=== Final Balances ===");
        for (i, user) in users.iter().enumerate() {
            let balance = token.get_shares(user);
            println!("User {} balance: {}", i + 1, balance);
            
            // Users 1, 3, 5 should have 0 balance
            if i % 2 == 0 {
                assert_eq!(balance, 0, "User {} should have 0 balance", i + 1);
            } else {
                // Users 2, 4 should have their remaining balance after partial withdrawal
                let expected = ((i + 1) * 1000) - ((i + 1) * 200);
                assert_eq!(balance, expected as u64, "User {} should have {} balance", i + 1, expected);
            }
        }

        println!("====== Multiple Users Deposit and Withdraw Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_vault_share_calculation() -> Result<()> {
        println!("\n====== Running Vault Share Calculation Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // First user gets 1:1 shares (1000 shares for 1000 tokens)
        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> First user depositing");
        println!("Amount: 1000");
        println!("Address: {:02x?}", user1);
        
        let transfer = token.deposit(1000, user1.clone())?;
        println!("First deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After First Deposit");
        print_user_balance(&token, &user1, "User 1");
        assert_eq!(token.get_shares(&user1), 1000, "First user should get 1:1 shares");

        // Simulate value increase: total_deposits = 1000, total_supply = 1000
        // Second user deposits same amount but gets fewer shares due to increased value
        let user2 = vec![2, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Second user depositing same amount after value increase");
        println!("Amount: 1000");
        println!("Address: {:02x?}", user2);
        
        let transfer = token.deposit(1000, user2.clone())?;
        println!("Second deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After Second Deposit");
        print_user_balance(&token, &user2, "User 2");
        
        // Verify share calculations
        let user2_shares = token.get_shares(&user2);
        println!("\n>>> Share distribution analysis:");
        println!("User 1 shares: 1000 (from 1000 token deposit)");
        println!("User 2 shares: {} (from 1000 token deposit)", user2_shares);
        println!("Total supply: {}", token.total_supply.borrow());
        println!("Total deposits: {}", token.total_deposits.borrow());

        // Third user deposits with double the amount
        let user3 = vec![3, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Third user depositing double amount");
        println!("Amount: 2000");
        println!("Address: {:02x?}", user3);
        
        let transfer = token.deposit(2000, user3.clone())?;
        println!("Third deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After Third Deposit");
        print_user_balance(&token, &user3, "User 3");

        // Verify proportional share distribution
        println!("\n>>> Final share distribution:");
        println!("User 1: {} shares", token.get_shares(&user1));
        println!("User 2: {} shares", token.get_shares(&user2));
        println!("User 3: {} shares", token.get_shares(&user3));
        
        // Test withdrawal with updated value
        println!("\n>>> Testing withdrawal with updated value");
        let user1_shares = token.get_shares(&user1);
        let (shares_transfer, token_transfer) = token.withdraw(user1_shares, user1.clone())?;
        println!("User 1 withdrawal result:");
        println!("Shares burned: {}", shares_transfer.value);
        println!("Tokens returned: {}", token_transfer.value);
        print_token_state(&token, "After Withdrawal");

        println!("====== Vault Share Calculation Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_share_calculation_with_appreciation() -> Result<()> {
        println!("\n====== Running Share Calculation with Appreciation Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // First user deposits 1000 tokens when vault is empty
        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> First user depositing 1000 tokens");
        let _transfer = token.deposit(1000, user1.clone())?;
        print_token_state(&token, "After First Deposit");
        print_user_balance(&token, &user1, "User 1");
        
        // Verify first user gets 1:1 shares
        assert_eq!(token.get_shares(&user1), 1000, "First user should get 1000 shares for 1000 tokens");
        
        // Simulate vault appreciation: total_deposits stays 1000, but value is now 2000
        // We do this by manually updating total_deposits
        *token.total_deposits.borrow_mut() = 2000;
        println!("\n>>> Simulating vault appreciation");
        println!("Original deposits: 1000");
        println!("New vault value: 2000");
        print_token_state(&token, "After Value Appreciation");

        // Second user deposits same amount (1000) after appreciation
        let user2 = vec![2, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Second user depositing 1000 tokens after appreciation");
        let _transfer = token.deposit(1000, user2.clone())?;
        print_token_state(&token, "After Second Deposit");
        print_user_balance(&token, &user2, "User 2");
        
        // Second user should get fewer shares for same deposit
        let user2_shares = token.get_shares(&user2);
        println!("\n>>> Share Analysis:");
        println!("User 1: 1000 shares for 1000 tokens (at vault value 1000)");
        println!("User 2: {} shares for 1000 tokens (at vault value 2000)", user2_shares);
        assert!(user2_shares < 1000, "Second user should get fewer shares due to appreciation");
        
        // Calculate expected shares for second user:
        // shares = deposit_amount * total_supply / total_deposits
        // shares = 1000 * 1000 / 2000 = 500
        assert_eq!(user2_shares, 500, "Second user should get 500 shares for 1000 tokens");

        // Verify withdrawal amounts
        println!("\n>>> Testing withdrawals after appreciation");
        
        // User 1 withdraws all shares
        let (_shares_transfer1, token_transfer1) = token.withdraw(1000, user1.clone())?;
        println!("User 1 withdrawal (1000 shares):");
        println!("Tokens returned: {}", token_transfer1.value);
        
        // User 2 withdraws all shares
        let (_shares_transfer2, token_transfer2) = token.withdraw(user2_shares, user2.clone())?;
        println!("User 2 withdrawal ({} shares):", user2_shares);
        println!("Tokens returned: {}", token_transfer2.value);
        
        // Verify proportional withdrawal amounts
        // User 1 (1000 shares) should get 2000 tokens (2/3 of the vault)
        // User 2 (500 shares) should get 1000 tokens (1/3 of the vault)
        assert_eq!(token_transfer1.value, 2000, "User 1 should get 2000 tokens for 1000 shares");
        assert_eq!(token_transfer2.value, 1000, "User 2 should get 1000 tokens for 500 shares");

        println!("====== Share Calculation with Appreciation Test Complete ======\n");
        Ok(())
    }
} 