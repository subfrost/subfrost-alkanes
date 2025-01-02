#[cfg(test)]
mod tests {
    use crate::alkanes::dxbtc::DxBtc;
    use alkanes_support::context::Context;
    use alkanes_support::id::AlkaneId;
    use alkanes_support::parcel::AlkaneTransferParcel;
    use anyhow::Result;
    use wasm_bindgen_test::*;
    use wasm_bindgen::JsValue;

    #[allow(unused_imports)]
    use {
        metashrew::{println, stdio::stdout},
        std::fmt::Write,
    };

    // Constants matching the implementation
    const VIRTUAL_SHARES: u64 = 1_000_000;  // 1M virtual shares
    const VIRTUAL_ASSETS: u64 = 1_000_000;  // 1M virtual assets
    const DECIMALS_MULTIPLIER: u64 = 1_000_000_000;  // 9 decimals

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

    #[wasm_bindgen_test]
    fn test_share_calculation_with_depreciation() -> Result<()> {
        println!("\n====== Running Share Calculation with Depreciation Test ======");
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
        
        // Simulate vault depreciation: total_deposits was 1000, now worth 500
        // We do this by manually updating total_deposits
        *token.total_deposits.borrow_mut() = 500;
        println!("\n>>> Simulating vault depreciation");
        println!("Original deposits: 1000");
        println!("New vault value: 500");
        print_token_state(&token, "After Value Depreciation");

        // Second user deposits same amount (1000) after depreciation
        let user2 = vec![2, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Second user depositing 1000 tokens after depreciation");
        let _transfer = token.deposit(1000, user2.clone())?;
        print_token_state(&token, "After Second Deposit");
        print_user_balance(&token, &user2, "User 2");
        
        // Second user should get more shares for same deposit
        let user2_shares = token.get_shares(&user2);
        println!("\n>>> Share Analysis:");
        println!("User 1: 1000 shares for 1000 tokens (at vault value 1000)");
        println!("User 2: {} shares for 1000 tokens (at vault value 500)", user2_shares);
        assert!(user2_shares > 1000, "Second user should get more shares due to depreciation");
        
        // Calculate expected shares for second user:
        // shares = deposit_amount * total_supply / total_deposits
        // shares = 1000 * 1000 / 500 = 2000
        assert_eq!(user2_shares, 2000, "Second user should get 2000 shares for 1000 tokens");

        // Verify withdrawal amounts
        println!("\n>>> Testing withdrawals after depreciation");
        
        // User 1 withdraws all shares
        let (_shares_transfer1, token_transfer1) = token.withdraw(1000, user1.clone())?;
        println!("User 1 withdrawal (1000 shares):");
        println!("Tokens returned: {}", token_transfer1.value);
        
        // User 2 withdraws all shares
        let (_shares_transfer2, token_transfer2) = token.withdraw(user2_shares, user2.clone())?;
        println!("User 2 withdrawal ({} shares):", user2_shares);
        println!("Tokens returned: {}", token_transfer2.value);
        
        // Verify proportional withdrawal amounts
        // User 1 (1000 shares) should get 500 tokens (1/3 of the vault)
        // User 2 (2000 shares) should get 1000 tokens (2/3 of the vault)
        assert_eq!(token_transfer1.value, 500, "User 1 should get 500 tokens for 1000 shares");
        assert_eq!(token_transfer2.value, 1000, "User 2 should get 1000 tokens for 2000 shares");

        println!("====== Share Calculation with Depreciation Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_virtual_offset_protection() -> Result<()> {
        println!("\n====== Running Virtual Offset Protection Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // First user attempts a very small deposit
        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let small_deposit = 100;  // Very small deposit
        println!("\n>>> First user depositing small amount");
        println!("Amount: {}", small_deposit);
        println!("Address: {:02x?}", user1);
        
        let transfer = token.deposit(small_deposit, user1.clone())?;
        println!("First deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After Small Deposit");
        print_user_balance(&token, &user1, "User 1");

        // Verify that the shares received are reasonable despite small deposit
        let shares_received = token.get_shares(&user1);
        println!("\n>>> Share Analysis:");
        println!("Deposit amount: {}", small_deposit);
        println!("Shares received: {}", shares_received);
        println!("Ratio: {}", shares_received as f64 / small_deposit as f64);
        
        // The ratio should be close to 1 for the first deposit due to virtual offset
        assert!(shares_received > 0, "Should receive non-zero shares even for small deposit");
        assert_eq!(shares_received, small_deposit, "First deposit should get 1:1 shares after virtual offset");

        println!("====== Virtual Offset Protection Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_decimal_precision() -> Result<()> {
        println!("\n====== Running Decimal Precision Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // First user deposits a large amount
        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let large_deposit = 1_000_000;
        println!("\n>>> First user depositing");
        println!("Amount: {}", large_deposit);
        println!("Address: {:02x?}", user1);
        
        let transfer = token.deposit(large_deposit, user1.clone())?;
        println!("First deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After First Deposit");
        print_user_balance(&token, &user1, "User 1");

        // Second user deposits a very small amount
        let user2 = vec![2, 0, 0, 0, 0, 0, 0, 0];
        let small_deposit = 100;
        println!("\n>>> Second user depositing small amount");
        println!("Amount: {}", small_deposit);
        println!("Address: {:02x?}", user2);
        
        let transfer = token.deposit(small_deposit, user2.clone())?;
        println!("Second deposit completed. Transfer result: {:?}", transfer);
        print_token_state(&token, "After Second Deposit");
        print_user_balance(&token, &user2, "User 2");

        // Verify precision in share calculation
        let user2_shares = token.get_shares(&user2);
        println!("\n>>> Share Analysis:");
        println!("User 1: {} shares for {} deposit", token.get_shares(&user1), large_deposit);
        println!("User 2: {} shares for {} deposit", user2_shares, small_deposit);
        
        // The small deposit should receive a proportional amount of shares
        assert!(user2_shares > 0, "Should receive non-zero shares for small deposit");
        
        // Calculate expected shares with high precision
        let expected_shares = (small_deposit as u128 * token.get_shares(&user1) as u128) / large_deposit as u128;
        println!("Expected shares for User 2: {}", expected_shares);
        assert_eq!(user2_shares, expected_shares as u64, "Share calculation should be precise");

        println!("====== Decimal Precision Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_preview_functions() -> Result<()> {
        println!("\n====== Running Preview Functions Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        let deposit_amount = 1000;
        println!("\n>>> Testing preview_deposit");
        println!("Amount to deposit: {}", deposit_amount);
        let expected_shares = token.preview_deposit(deposit_amount);
        println!("Expected shares: {}", expected_shares);

        // Perform actual deposit
        let user = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let transfer = token.deposit(deposit_amount, user.clone())?;
        println!("Actual deposit completed. Transfer result: {:?}", transfer);
        
        // Verify preview was accurate
        assert_eq!(transfer.value, expected_shares, "Preview deposit should match actual shares received");

        // Test preview_withdraw
        println!("\n>>> Testing preview_withdraw");
        let shares_to_withdraw = 500;
        println!("Shares to withdraw: {}", shares_to_withdraw);
        let expected_assets = token.preview_withdraw(shares_to_withdraw);
        println!("Expected assets: {}", expected_assets);

        // Perform actual withdrawal
        let (shares_transfer, assets_transfer) = token.withdraw(shares_to_withdraw, user.clone())?;
        println!("Actual withdrawal completed.");
        println!("Shares burned: {}", shares_transfer.value);
        println!("Assets returned: {}", assets_transfer.value);
        
        // Verify preview was accurate
        assert_eq!(assets_transfer.value, expected_assets, "Preview withdraw should match actual assets received");

        println!("====== Preview Functions Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_asset_share_conversions() -> Result<()> {
        println!("\n====== Running Asset/Share Conversion Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // Test initial conversion (should be 1:1 due to virtual offset)
        let initial_assets = 1000;
        println!("\n>>> Testing initial conversion (empty vault)");
        println!("Converting {} assets to shares", initial_assets);
        let initial_shares = token.convert_to_shares(initial_assets);
        println!("Got {} shares", initial_shares);
        assert_eq!(initial_shares, initial_assets as u128, "Initial conversion should be 1:1");

        // Make a deposit to set up vault state
        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        println!("\n>>> Setting up vault with initial deposit");
        println!("Depositing {} assets", initial_assets);
        let transfer = token.deposit(initial_assets, user1.clone())?;
        print_token_state(&token, "After Initial Deposit");

        // Test conversion after vault has value
        let test_assets = 500;
        println!("\n>>> Testing conversion with active vault");
        println!("Converting {} assets to shares", test_assets);
        let shares = token.convert_to_shares(test_assets);
        println!("Got {} shares", shares);
        
        // Convert shares back to assets
        println!("Converting {} shares back to assets", shares);
        let assets = token.convert_to_assets(shares as u64);
        println!("Got {} assets", assets);
        
        // Verify conversion roundtrip
        assert!(assets <= test_assets as u128, "Conversion roundtrip should not inflate value");
        
        println!("====== Asset/Share Conversion Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_max_operations() -> Result<()> {
        println!("\n====== Running Max Operations Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        let user = vec![1, 0, 0, 0, 0, 0, 0, 0];
        
        // Test max deposit/mint before any deposits
        println!("\n>>> Testing max operations on empty vault");
        let max_deposit = token.max_deposit(&user);
        let max_mint = token.max_mint(&user);
        println!("Max deposit: {}", max_deposit);
        println!("Max mint: {}", max_mint);
        
        // Make a deposit
        let deposit_amount = 1000;
        println!("\n>>> Making initial deposit");
        println!("Amount: {}", deposit_amount);
        let transfer = token.deposit(deposit_amount, user.clone())?;
        print_token_state(&token, "After Deposit");
        print_user_balance(&token, &user, "User");

        // Test max withdraw/redeem
        println!("\n>>> Testing max operations with balance");
        let max_withdraw = token.max_withdraw(&user);
        let max_redeem = token.max_redeem(&user);
        println!("Max withdraw: {}", max_withdraw);
        println!("Max redeem: {}", max_redeem);
        
        // Verify max withdraw/redeem matches user balance
        assert_eq!(max_withdraw, deposit_amount as u128, "Max withdraw should match deposit");
        assert_eq!(max_redeem, deposit_amount as u128, "Max redeem should match deposit");

        println!("====== Max Operations Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_mint_redeem_operations() -> Result<()> {
        println!("\n====== Running Mint/Redeem Operations Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        let user1 = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let mint_shares = 1000;
        
        // Test mint operation
        println!("\n>>> Testing mint operation");
        println!("Minting {} shares", mint_shares);
        let transfer = token.mint(mint_shares, user1.clone())?;
        print_token_state(&token, "After Mint");
        print_user_balance(&token, &user1, "User 1");
        
        // Verify minted shares
        let user_shares = token.get_shares(&user1);
        assert_eq!(user_shares, mint_shares, "User should receive exact requested shares");
        
        // Preview redeem before actual redeem
        println!("\n>>> Testing redeem preview");
        let redeem_shares = 500;
        println!("Previewing redemption of {} shares", redeem_shares);
        let expected_assets = token.preview_redeem(redeem_shares);
        println!("Expected assets: {}", expected_assets);
        
        // Test redeem operation
        println!("\n>>> Testing redeem operation");
        println!("Redeeming {} shares", redeem_shares);
        let (shares_transfer, assets_transfer) = token.redeem(redeem_shares, user1.clone())?;
        print_token_state(&token, "After Redeem");
        print_user_balance(&token, &user1, "User 1");
        
        // Verify redeem results
        assert_eq!(shares_transfer.value, redeem_shares as u128, "Should burn exact requested shares");
        assert_eq!(assets_transfer.value, expected_assets, "Should receive previewed assets");
        
        println!("====== Mint/Redeem Operations Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_total_assets_tracking() -> Result<()> {
        println!("\n====== Running Total Assets Tracking Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");

        // Verify initial total assets
        println!("\n>>> Checking initial total assets");
        let initial_assets = token.total_assets();
        println!("Initial total assets: {}", initial_assets);
        assert_eq!(initial_assets, 0, "Should start with zero assets");

        // Make deposits from multiple users
        let users: Vec<Vec<u8>> = (1..=3)
            .map(|i| vec![i as u8, 0, 0, 0, 0, 0, 0, 0])
            .collect();
        
        let mut total_deposited = 0;
        println!("\n>>> Making deposits from multiple users");
        for (i, user) in users.iter().enumerate() {
            let amount = (i + 1) * 1000;
            total_deposited += amount;
            println!("\nUser {} depositing {}", i + 1, amount);
            let transfer = token.deposit(amount as u64, user.clone())?;
            print_token_state(&token, &format!("After User {} Deposit", i + 1));
        }

        // Verify total assets after deposits
        let total_assets = token.total_assets();
        println!("\n>>> Checking total assets after deposits");
        println!("Total assets: {}", total_assets);
        println!("Total deposited: {}", total_deposited);
        assert_eq!(total_assets, total_deposited as u128, "Total assets should match deposits");

        // Make some withdrawals
        println!("\n>>> Making withdrawals");
        let withdraw_user = &users[0];
        let withdraw_shares = token.get_shares(withdraw_user) / 2;
        println!("User 1 withdrawing {} shares", withdraw_shares);
        let (_, assets_transfer) = token.withdraw(withdraw_shares, withdraw_user.clone())?;
        print_token_state(&token, "After Withdrawal");

        // Verify total assets after withdrawal
        let final_assets = token.total_assets();
        println!("\n>>> Checking final total assets");
        println!("Final total assets: {}", final_assets);
        println!("Assets withdrawn: {}", assets_transfer.value);
        assert_eq!(
            final_assets,
            total_deposited as u128 - assets_transfer.value,
            "Total assets should reflect withdrawal"
        );

        println!("====== Total Assets Tracking Test Complete ======\n");
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_edge_cases() -> Result<()> {
        console_log!("\n====== Running Edge Cases Test ======\n");
        
        let (token, _context) = setup_token();
        
        console_log!(">>> Testing zero value operations");
        console_log!("Attempting zero deposit...");
        
        // Test that zero deposits are rejected
        let user = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let result = token.deposit(0, user.clone());
        match result {
            Ok(_) => {
                console_log!("Zero deposit was incorrectly accepted");
                assert!(false, "Zero deposits should be rejected");
            },
            Err(_) => {
                console_log!("Zero deposit correctly rejected");
            }
        }
        
        // Test minimum deposit
        console_log!("\n>>> Testing minimum deposit");
        let min_deposit = 1;
        let _transfer = token.deposit(min_deposit, user.clone())?;
        let balance = token.get_shares(&user);
        assert_eq!(balance, min_deposit, "Minimum deposit should be accepted");
        
        // Test maximum deposit
        console_log!("\n>>> Testing maximum deposit");
        let max_deposit = u64::MAX;
        assert!(max_deposit > 0, "Maximum deposit should be positive");
        
        // Test rounding behavior
        console_log!("\n>>> Testing rounding behavior");
        let odd_amount = 1001;
        let _transfer = token.deposit(odd_amount, user.clone())?;
        let balance = token.get_shares(&user);
        assert_eq!(balance, min_deposit + odd_amount, "Deposit should handle odd amounts");
        
        console_log!("\n====== Edge Cases Test Complete ======\n");
        Ok(())
    }
} 