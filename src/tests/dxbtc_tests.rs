#[cfg(test)]
mod tests {
    use dx_btc::{DxBtc, SHARE_PRECISION_OFFSET, VIRTUAL_ASSETS, VIRTUAL_SHARES, MIN_DEPOSIT, MOCK_CONTEXT};
    use alkanes_support::context::Context;
    use alkanes_support::parcel::AlkaneTransfer;
    use std::io::Cursor;
    use anyhow::Result;
    use wasm_bindgen_test::wasm_bindgen_test;
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static TEST_DX_BTC: RefCell<Option<Rc<DxBtc>>> = RefCell::new(None);
    }

    fn reset_dx_btc() {
        TEST_DX_BTC.with(|dx_btc| {
            *dx_btc.borrow_mut() = Some(Rc::new(DxBtc::new()));
        });
    }

    fn get_dx_btc() -> Rc<DxBtc> {
        TEST_DX_BTC.with(|dx_btc| {
            if dx_btc.borrow().is_none() {
                *dx_btc.borrow_mut() = Some(Rc::new(DxBtc::new()));
            }
            dx_btc.borrow().as_ref().unwrap().clone()
        })
    }

    fn setup_context() -> Result<Context> {
        let buffer = vec![0u8; 1024];
        let mut cursor = Cursor::new(buffer);
        Context::parse(&mut cursor)
    }

    fn setup_deposit_context(amount: u128, has_balance: bool) -> Result<Context> {
        let mut context = setup_context()?;
        let dx_btc = get_dx_btc();
        let deposit_token = dx_btc.deposit_token.clone();
        
        // Only add transfer if user has balance
        if has_balance {
            let deposit_transfer = AlkaneTransfer {
                id: deposit_token.clone(),
                value: amount,
            };
            context.incoming_alkanes.0.push(deposit_transfer.clone());
            
            // Set up initial balance
            dx_btc.set_balance(&deposit_transfer.id, &dx_btc.deposit_token, amount)?;
        }
        Ok(context)
    }

    #[wasm_bindgen_test]
    async fn test_deposit_without_balance() -> Result<()> {
        reset_dx_btc();
        let context = setup_deposit_context(VIRTUAL_ASSETS, false)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context.clone()));

        let dx_btc = get_dx_btc();
        
        // Attempt deposit without balance should fail
        let result = dx_btc.deposit(&context);
        assert!(result.is_err(), "Deposit without balance should fail");
        assert!(result.unwrap_err().to_string().contains("Deposit transfer not found"));
        
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_withdraw_without_shares() -> Result<()> {
        reset_dx_btc();
        let mut context = setup_context()?;
        let dx_btc = get_dx_btc();
        
        // Try to withdraw shares we don't have
        let share_transfer = AlkaneTransfer {
            id: context.myself.clone(),
            value: VIRTUAL_SHARES * SHARE_PRECISION_OFFSET,
        };
        context.incoming_alkanes.0.push(share_transfer);
        
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context.clone()));
        
        // Verify we have no shares
        let initial_shares = dx_btc.get_shares(&context.myself)?;
        assert_eq!(initial_shares, 0, "Should have no shares initially");
        
        // Attempt withdraw without shares should fail
        let result = dx_btc.withdraw(&context);
        assert!(result.is_err(), "Withdraw without shares should fail");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Insufficient shares for withdrawal"), 
            "Wrong error message: {}", err);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_precision_with_small_deposit() -> Result<()> {
        reset_dx_btc();
        // Test with minimum deposit to verify precision
        let context = setup_deposit_context(MIN_DEPOSIT, true)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context.clone()));

        let dx_btc = get_dx_btc();
        
        // Verify initial balance
        let deposit_transfer = context.incoming_alkanes.0.first().unwrap();
        let initial_balance = dx_btc.get_balance(&deposit_transfer.id)?;
        assert_eq!(initial_balance, MIN_DEPOSIT, "Initial balance not set correctly");
        
        let share_transfer = dx_btc.deposit(&context)?;
        
        // Even minimum deposit should get meaningful shares due to virtual offset
        assert!(share_transfer.value >= SHARE_PRECISION_OFFSET, 
            "Small deposit got too few shares: {}", share_transfer.value);
            
        // Verify shares are properly scaled
        assert_eq!(share_transfer.value % SHARE_PRECISION_OFFSET, 0,
            "Shares not properly precision-scaled: {}", share_transfer.value);
            
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_deposit_withdraw_precision() -> Result<()> {
        reset_dx_btc();
        // First make a deposit
        let deposit_amount = VIRTUAL_ASSETS;
        let context = setup_deposit_context(deposit_amount, true)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context.clone()));

        let dx_btc = get_dx_btc();
        
        // Verify initial balance
        let deposit_transfer = context.incoming_alkanes.0.first().unwrap();
        let initial_balance = dx_btc.get_balance(&deposit_transfer.id)?;
        assert_eq!(initial_balance, deposit_amount, "Initial balance not set correctly");
        
        let share_transfer = dx_btc.deposit(&context)?;
        let initial_shares = share_transfer.value;
        
        // Now withdraw half the shares
        let mut withdraw_context = setup_context()?;
        let half_shares = initial_shares / 2;
        withdraw_context.incoming_alkanes.0.push(AlkaneTransfer {
            id: withdraw_context.myself.clone(),
            value: half_shares,
        });
        
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(withdraw_context.clone()));
        
        let (shares_out, assets_out) = dx_btc.withdraw(&withdraw_context)?;
        
        // Verify precision in withdrawal
        assert_eq!(shares_out.value, half_shares, 
            "Share withdrawal amount mismatch");
        assert_eq!(assets_out.value, deposit_amount / 2, 
            "Asset withdrawal amount mismatch");
            
        // Verify remaining shares are properly scaled
        let remaining_shares = initial_shares - shares_out.value;
        assert_eq!(remaining_shares % SHARE_PRECISION_OFFSET, 0,
            "Remaining shares not properly precision-scaled: {}", remaining_shares);
            
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_virtual_offset_exchange_rate() -> Result<()> {
        reset_dx_btc();
        let dx_btc = get_dx_btc();
        
        // Make two deposits of different sizes
        let deposits = vec![MIN_DEPOSIT, MIN_DEPOSIT * 2];
        let mut share_ratios = Vec::new();
        
        for amount in deposits {
            let context = setup_deposit_context(amount, true)?;
            MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context.clone()));
            
            // Verify initial balance
            let deposit_transfer = context.incoming_alkanes.0.first().unwrap();
            let initial_balance = dx_btc.get_balance(&deposit_transfer.id)?;
            assert_eq!(initial_balance, amount, "Initial balance not set correctly");
            
            let shares = dx_btc.deposit(&context)?.value;
            let ratio = shares as f64 / amount as f64;
            share_ratios.push(ratio);
        }
        
        // Virtual offset should keep ratios similar
        let ratio_diff = (share_ratios[0] - share_ratios[1]).abs();
        assert!(ratio_diff < 0.01, 
            "Virtual offset not maintaining consistent exchange rate, ratio difference: {}", ratio_diff);
            
        Ok(())
    }
} 
