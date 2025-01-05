#[cfg(test)]
mod tests {
    use dx_btc::{DxBtc, SHARE_PRECISION_OFFSET, VIRTUAL_ASSETS, MIN_DEPOSIT, MOCK_CONTEXT};
    use alkanes_support::context::Context;
    use alkanes_support::id::AlkaneId;
    use alkanes_support::parcel::AlkaneTransfer;
    use std::io::Cursor;
    use anyhow::Result;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn setup_context() -> Result<Context> {
        // Initialize with a 1KB buffer
        let buffer = vec![0u8; 1024];
        let mut cursor = Cursor::new(buffer);
        Context::parse(&mut cursor)
    }

    fn setup_deposit_context(amount: u128) -> Result<Context> {
        let mut context = setup_context()?;
        let deposit_token = AlkaneId::new(1, 2); // BTC token
        let deposit_transfer = AlkaneTransfer {
            id: deposit_token,
            value: amount,
        };
        context.incoming_alkanes.0.push(deposit_transfer);
        Ok(context)
    }

    #[wasm_bindgen_test]
    async fn test_deposit_flow() -> Result<()> {
        let context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let share_transfer = dx_btc.deposit()?;
        
        assert_eq!(share_transfer.value, MIN_DEPOSIT * SHARE_PRECISION_OFFSET);
        assert_eq!(*dx_btc.total_supply.borrow(), MIN_DEPOSIT * SHARE_PRECISION_OFFSET);
        assert_eq!(*dx_btc.total_deposits.borrow(), MIN_DEPOSIT);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_withdraw_flow() -> Result<()> {
        // First make a deposit
        let context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let share_transfer = dx_btc.deposit()?;
        
        // Now withdraw all shares
        let mut withdraw_context = setup_context()?;
        withdraw_context.incoming_alkanes.0.push(AlkaneTransfer {
            id: withdraw_context.myself.clone(),
            value: share_transfer.value,
        });
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(withdraw_context));

        let (shares_transfer, assets_transfer) = dx_btc.withdraw()?;
        
        assert_eq!(shares_transfer.value, share_transfer.value);
        assert_eq!(assets_transfer.value, MIN_DEPOSIT);
        assert_eq!(*dx_btc.total_supply.borrow(), 0);
        assert_eq!(*dx_btc.total_deposits.borrow(), 0);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_share_calculation_safety() -> Result<()> {
        let context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let initial_shares = dx_btc.deposit()?.value;
        
        // Second deposit
        let context = setup_deposit_context(MIN_DEPOSIT * 2)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let second_shares = dx_btc.deposit()?.value;
        
        // Verify share ratio is maintained
        assert_eq!(second_shares, initial_shares * 2);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_withdrawal_safety() -> Result<()> {
        // First make a deposit
        let context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let share_transfer = dx_btc.deposit()?;
        
        // Now withdraw half the shares
        let mut withdraw_context = setup_context()?;
        withdraw_context.incoming_alkanes.0.push(AlkaneTransfer {
            id: withdraw_context.myself.clone(),
            value: share_transfer.value / 2,
        });
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(withdraw_context));

        let (shares_transfer, assets_transfer) = dx_btc.withdraw()?;
        
        assert_eq!(shares_transfer.value, share_transfer.value / 2);
        assert_eq!(assets_transfer.value, MIN_DEPOSIT / 2);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_preview_operations() -> Result<()> {
        let dx_btc = DxBtc::new();
        
        // Test preview deposit
        let deposit_amount = MIN_DEPOSIT;
        let expected_shares = dx_btc.preview_deposit(deposit_amount)?;
        
        // Make actual deposit
        let context = setup_deposit_context(deposit_amount)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let actual_shares = dx_btc.deposit()?.value;
        assert_eq!(actual_shares, expected_shares);
        
        // Test preview withdraw
        let expected_assets = dx_btc.preview_withdraw(actual_shares)?;
        assert_eq!(expected_assets, deposit_amount);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_inflation_attack_resistance() -> Result<()> {
        let dx_btc = DxBtc::new();
        
        // Try a deposit below minimum - should fail
        let tiny_deposit = MIN_DEPOSIT / 2;
        let context = setup_deposit_context(tiny_deposit)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let result = dx_btc.deposit();
        assert!(result.is_err());
        
        // Initial deposit at minimum
        let context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let initial_shares = dx_btc.deposit()?.value;
        
        // Large follow-up deposit
        let context = setup_deposit_context(MIN_DEPOSIT * 1000)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let large_shares = dx_btc.deposit()?.value;
        
        // Verify share ratio is maintained
        let expected_shares = initial_shares * 1000;
        let share_deviation = ((large_shares as f64 - expected_shares as f64) / expected_shares as f64 * 100.0).abs();
        assert!(share_deviation < 1.0, "Share price deviation too large: {:.0}%", share_deviation);
        
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_virtual_share_impact() -> Result<()> {
        let dx_btc = DxBtc::new();
        
        // Make initial deposit equal to virtual assets
        let context = setup_deposit_context(VIRTUAL_ASSETS)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let shares = dx_btc.deposit()?.value;
        
        // Due to virtual shares/assets, should get approximately half the shares
        let expected_shares = VIRTUAL_ASSETS * SHARE_PRECISION_OFFSET / 2;
        let share_deviation = ((shares as f64 - expected_shares as f64) / expected_shares as f64 * 100.0).abs();
        assert!(share_deviation < 1.0, "Share price deviation too large: {:.0}%", share_deviation);
        
        Ok(())
    }
} 
