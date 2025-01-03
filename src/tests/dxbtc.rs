#[cfg(any(feature = "test-utils", test))]
pub mod tests {
    use dx_btc::DxBtc;
    use alkanes_support::context::Context;
    use alkanes_support::parcel::{AlkaneTransfer, AlkaneTransferParcel};
    use alkanes_support::id::AlkaneId;
    use alkanes_runtime::runtime::AlkaneResponder;
    use alkanes_support::response::CallResponse;
    use anyhow::Result;
    use wasm_bindgen_test::*;
    use crate::tests::helpers::AlkaneTest;

    wasm_bindgen_test_configure!(run_in_browser);

    impl AlkaneTest for DxBtc {
        fn get_deposit_token(&self) -> AlkaneId {
            self.deposit_token.borrow().clone().expect("Deposit token not initialized")
        }
        
        fn set_mock_context(context: Context) {
            thread_local! {
                static MOCK_CONTEXT: std::cell::RefCell<Option<Context>> = std::cell::RefCell::new(None);
            }
            
            MOCK_CONTEXT.with(|ctx| {
                *ctx.borrow_mut() = Some(context);
            });
        }
    }

    fn create_test_context(myself: AlkaneId, caller: AlkaneId) -> Context {
        Context {
            myself,
            caller,
            inputs: vec![],
            incoming_alkanes: AlkaneTransferParcel(vec![]),
            vout: 0,
        }
    }

    fn setup_token() -> (DxBtc, Context) {
        let token = DxBtc::default();
        let deposit_token = AlkaneId::new(1, 2);
        *token.deposit_token.borrow_mut() = Some(deposit_token);
        
        let context = create_test_context(
            AlkaneId::new(1, 1), // myself
            AlkaneId::new(1, 2)  // caller
        );
        DxBtc::set_mock_context(context.clone());
        
        (token, context)
    }

    fn setup_incoming_deposit(context: &mut Context, amount: u128) {
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: AlkaneId::new(1, 2), // deposit token
            value: amount,
        });
    }

    #[wasm_bindgen_test]
    pub fn test_deposit_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        let caller = AlkaneId::new(1, 3);
        let deposit_amount: u128 = 1000;
        
        setup_incoming_deposit(&mut context, deposit_amount);
        context.inputs = vec![1]; // deposit opcode
        DxBtc::set_mock_context(context);
        
        token.execute()?;
        
        // Verify state
        let caller_key = DxBtc::get_key_for_alkane_id(&caller);
        assert_eq!(token.get_shares(&caller_key), deposit_amount, "Caller should have correct shares");
        assert_eq!(*token.total_supply.borrow(), deposit_amount, "Total supply should match deposit");
        assert_eq!(*token.total_deposits.borrow(), deposit_amount, "Total deposits should match deposit");

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_withdraw_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        let caller = AlkaneId::new(1, 3);
        let deposit_amount: u128 = 1000;
        
        // Initial deposit
        setup_incoming_deposit(&mut context, deposit_amount);
        context.inputs = vec![1]; // deposit opcode
        DxBtc::set_mock_context(context.clone());
        token.execute()?;
        
        // Withdraw
        let shares_to_withdraw: u128 = 500;
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares_to_withdraw,
        });
        context.inputs = vec![2]; // withdraw opcode
        DxBtc::set_mock_context(context);
        token.execute()?;
        
        // Verify state
        let caller_key = DxBtc::get_key_for_alkane_id(&caller);
        assert_eq!(token.get_shares(&caller_key), deposit_amount - shares_to_withdraw, 
            "Caller should have correct remaining shares");

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_deposit_safety() -> Result<()> {
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

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_share_calculation_safety() -> Result<()> {
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

        // The share ratio should be determined by the vault state
        let shares = response.alkanes.0[0].value;
        let expected_shares = token.preview_deposit(1000)?;
        assert_eq!(shares, expected_shares, "Actual shares should match preview");

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_preview_operations() -> Result<()> {
        let (token, mut context) = setup_token();

        // Test preview_deposit
        let amount = 1000;
        context.inputs = vec![5, amount];  // preview_deposit opcode
        DxBtc::set_mock_context(context.clone());
        let response = token.execute()?;
        let preview_shares = u128::from_le_bytes(response.data.try_into().unwrap());
        assert_eq!(preview_shares, amount, "First deposit preview should be 1:1");

        // Test preview_mint
        context.inputs = vec![6, amount];  // preview_mint opcode
        DxBtc::set_mock_context(context.clone());
        let response = token.execute()?;
        let preview_assets = u128::from_le_bytes(response.data.try_into().unwrap());
        assert_eq!(preview_assets, amount, "First mint preview should be 1:1");

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_withdrawal_safety() -> Result<()> {
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

        Ok(())
    }

    #[wasm_bindgen_test]
    pub fn test_state_consistency() -> Result<()> {
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

        Ok(())
    }
} 
