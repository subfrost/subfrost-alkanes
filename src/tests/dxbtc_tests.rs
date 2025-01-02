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

    #[wasm_bindgen_test]
    fn test_initialization() -> Result<()> {
        println!("\n====== Running Initialization Test ======");
        let (token, _) = setup_token();
        print_token_state(&token, "Initial State");
        
        // Verify initial state
        assert!(token.deposit_token.borrow().is_none(), "Deposit token should be None initially");
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
} 