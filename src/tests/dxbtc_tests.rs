#[cfg(test)]
mod tests {
    use dx_btc::DxBtc;
    use alkanes_support::context::Context;
    use alkanes_support::id::AlkaneId;
    use alkanes_support::parcel::{AlkaneTransfer, AlkaneTransferParcel};
    use anyhow::Result;
    use wasm_bindgen_test::*;

    fn setup_token() -> (DxBtc, Context) {
        let token = DxBtc::default();
        let context = Context {
            myself: AlkaneId::new(1, 1),
            inputs: vec![],
            incoming_alkanes: AlkaneTransferParcel::default(),
            caller: AlkaneId::new(1, 3),
            vout: 0,
        };
        
        // Initialize deposit token
        let deposit_token = AlkaneId::new(1, 2);
        *token.deposit_token.borrow_mut() = Some(deposit_token.clone());
        
        DxBtc::set_mock_context(context.clone());
        (token, context)
    }

    fn setup_incoming_deposit(context: &mut Context, amount: u128) {
        let deposit_token = AlkaneId::new(1, 2);
        context.incoming_alkanes.0.clear();  // Clear previous transfers
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: deposit_token,
            value: amount,
        });
        DxBtc::set_mock_context(context.clone());
    }

    #[wasm_bindgen_test]
    fn test_deposit_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // Test receive opcode
        context.inputs = vec![0];  // receive opcode
        DxBtc::set_mock_context(context.clone());
        let response = token.execute()?;
        assert!(response.alkanes.0.is_empty(), "Receive should return empty response");
        
        // Test deposit
        let deposit_amount = 1000;
        setup_incoming_deposit(&mut context, deposit_amount);
        
        context.inputs = vec![1];  // deposit opcode
        DxBtc::set_mock_context(context.clone());
        
        let response = token.execute()?;
        assert_eq!(response.alkanes.0.len(), 1, "Deposit should return one transfer");
        assert_eq!(response.alkanes.0[0].value, deposit_amount, "Should get 1:1 shares for first deposit");
        
        // Verify state
        let caller_key = DxBtc::get_key_for_alkane_id(&context.caller);
        assert_eq!(token.get_shares(&caller_key), deposit_amount, "Caller should have correct shares");
        assert_eq!(*token.total_supply.borrow(), deposit_amount, "Total supply should match deposit");
        assert_eq!(*token.total_deposits.borrow(), deposit_amount, "Total deposits should match deposit");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_deposit_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // Test deposit with zero amount
        setup_incoming_deposit(&mut context, 0);
        context.inputs = vec![1];  // deposit opcode
        DxBtc::set_mock_context(context.clone());
        assert!(token.execute().is_err(), "Should fail with zero amount deposit");

        // Test deposit with wrong token
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: AlkaneId::new(1, 99),  // Wrong token
            value: 1000,
        });
        DxBtc::set_mock_context(context.clone());
        assert!(token.execute().is_err(), "Should fail with wrong token");

        // Test multiple deposits from same user
        setup_incoming_deposit(&mut context, 1000);
        token.execute()?;
        
        setup_incoming_deposit(&mut context, 500);
        let response = token.execute()?;
        assert!(response.alkanes.0[0].value > 0, "Second deposit should give non-zero shares");
        
        let caller_key = DxBtc::get_key_for_alkane_id(&context.caller);
        assert!(token.get_shares(&caller_key) > 1000, "Balance should increase after second deposit");

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_share_calculation_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // Test first deposit (1:1 ratio)
        setup_incoming_deposit(&mut context, 1000);
        context.inputs = vec![1];
        DxBtc::set_mock_context(context.clone());
        let response = token.execute()?;
        assert_eq!(response.alkanes.0[0].value, 1000, "First deposit should be 1:1");

        // Test subsequent deposit
        setup_incoming_deposit(&mut context, 1000);
        let response = token.execute()?;
        assert!(response.alkanes.0[0].value > 0, "Subsequent deposit should give non-zero shares");
        
        // The share ratio should be determined by the vault state, not artificially constrained
        let shares = response.alkanes.0[0].value;
        let expected_shares = token.preview_deposit(1000)?;
        assert_eq!(shares, expected_shares, "Actual shares should match preview");

        // Test extreme values
        setup_incoming_deposit(&mut context, u128::MAX / 2);
        assert!(token.execute().is_err(), "Should handle extreme values safely");

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_withdraw_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // First deposit to have something to withdraw
        let deposit_amount = 1000;
        setup_incoming_deposit(&mut context, deposit_amount);
        context.inputs = vec![1];  // deposit opcode
        DxBtc::set_mock_context(context.clone());
        token.execute()?;
        
        // Now withdraw
        let shares_to_withdraw = 500;
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares_to_withdraw,
        });
        
        context.inputs = vec![2];  // withdraw opcode
        DxBtc::set_mock_context(context.clone());
        
        let response = token.execute()?;
        assert_eq!(response.alkanes.0.len(), 2, "Withdraw should return two transfers");
        
        // Verify state
        let caller_key = DxBtc::get_key_for_alkane_id(&context.caller);
        assert_eq!(token.get_shares(&caller_key), deposit_amount - shares_to_withdraw, 
            "Caller should have correct remaining shares");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_preview_operations() -> Result<()> {
        let (token, mut context) = setup_token();
        
        let amount = 1000;
        context.inputs = vec![3, amount];  // preview_deposit opcode
        DxBtc::set_mock_context(context.clone());
        
        let response = token.execute()?;
        let preview_shares = u128::from_le_bytes(response.data.try_into().unwrap());
        assert_eq!(preview_shares, amount, "First deposit preview should be 1:1");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_withdrawal_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // First deposit
        setup_incoming_deposit(&mut context, 1000);
        context.inputs = vec![1];
        DxBtc::set_mock_context(context.clone());
        token.execute()?;
        
        // Test withdrawal with insufficient shares
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: 2000,  // More than deposited
        });
        context.inputs = vec![2];  // withdraw opcode
        DxBtc::set_mock_context(context.clone());
        assert!(token.execute().is_err(), "Should fail with insufficient shares");

        // Test withdrawal with zero shares
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: 0,
        });
        DxBtc::set_mock_context(context.clone());
        assert!(token.execute().is_err(), "Should fail with zero shares");

        // Test full withdrawal
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: 1000,
        });
        DxBtc::set_mock_context(context.clone());
        let response = token.execute()?;
        assert_eq!(response.alkanes.0.len(), 2, "Full withdrawal should return two transfers");
        
        let caller_key = DxBtc::get_key_for_alkane_id(&context.caller);
        assert_eq!(token.get_shares(&caller_key), 0, "Balance should be zero after full withdrawal");
        
        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_state_consistency() -> Result<()> {
        let (token, mut context) = setup_token();
        
        // Multiple deposits from different users
        let first_user = context.caller.clone();
        setup_incoming_deposit(&mut context, 1000);
        context.inputs = vec![1];
        DxBtc::set_mock_context(context.clone());
        token.execute()?;
        
        // Second user deposit
        context.caller = AlkaneId::new(1, 4);
        setup_incoming_deposit(&mut context, 500);
        DxBtc::set_mock_context(context.clone());
        token.execute()?;
        
        // Verify total supply matches sum of balances
        let first_user_shares = token.get_shares(&DxBtc::get_key_for_alkane_id(&first_user));
        let second_user_shares = token.get_shares(&DxBtc::get_key_for_alkane_id(&context.caller));
        assert_eq!(*token.total_supply.borrow(), first_user_shares + second_user_shares,
            "Total supply should match sum of balances");
        
        // Verify total deposits is accurate
        assert_eq!(*token.total_deposits.borrow(), 1500,
            "Total deposits should be accurate");
            
        // Test state after failed operation
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: 1000,  // More than user has
        });
        context.inputs = vec![2];  // withdraw opcode
        DxBtc::set_mock_context(context.clone());
        assert!(token.execute().is_err());
        
        // Verify state hasn't changed after failed operation
        assert_eq!(*token.total_supply.borrow(), first_user_shares + second_user_shares,
            "Total supply should be unchanged after failed operation");
        assert_eq!(*token.total_deposits.borrow(), 1500,
            "Total deposits should be unchanged after failed operation");
        
        Ok(())
    }
} 
