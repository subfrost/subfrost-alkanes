#[cfg(test)]
mod tests {
    use dx_btc::{DxBtc, SHARE_PRECISION_OFFSET, VIRTUAL_ASSETS, VIRTUAL_SHARES, MIN_DEPOSIT, MOCK_CONTEXT};
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

    // Helper to verify raw storage values
    fn verify_raw_storage(dx_btc: &DxBtc, expected_supply: u128, expected_deposits: u128) {
        assert_eq!(*dx_btc.total_supply.borrow(), expected_supply, "Raw share supply mismatch");
        assert_eq!(*dx_btc.total_deposits.borrow(), expected_deposits, "Raw deposits mismatch");
    }

    #[wasm_bindgen_test]
    async fn test_initial_deposit_virtual_offset() -> Result<()> {
        let context = setup_deposit_context(VIRTUAL_ASSETS)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let share_transfer = dx_btc.deposit()?;
        
        // For initial deposit with amount = VIRTUAL_ASSETS:
        // shares = (VIRTUAL_ASSETS * VIRTUAL_SHARES / VIRTUAL_ASSETS) * SHARE_PRECISION_OFFSET
        // = VIRTUAL_SHARES * SHARE_PRECISION_OFFSET
        let expected_shares = VIRTUAL_SHARES * SHARE_PRECISION_OFFSET;
        assert_eq!(share_transfer.value, expected_shares, "Initial share calculation incorrect");
        
        // Verify raw storage
        verify_raw_storage(&dx_btc, VIRTUAL_SHARES, VIRTUAL_ASSETS);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_subsequent_deposit_scaling() -> Result<()> {
        // First deposit
        let context = setup_deposit_context(VIRTUAL_ASSETS)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let initial_shares = dx_btc.deposit()?.value;

        // Second deposit of same size
        let context = setup_deposit_context(VIRTUAL_ASSETS)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let second_shares = dx_btc.deposit()?.value;
        
        // Both deposits should get same shares due to virtual offset
        assert_eq!(second_shares, initial_shares, "Subsequent deposit shares mismatch");
        
        // Verify raw storage is doubled
        verify_raw_storage(&dx_btc, 
            2 * VIRTUAL_SHARES,  // Raw shares
            2 * VIRTUAL_ASSETS   // Raw assets
        );
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_precision_maintenance() -> Result<()> {
        let small_deposit = MIN_DEPOSIT + 1;
        let context = setup_deposit_context(small_deposit)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let shares = dx_btc.deposit()?.value;
        
        // Verify shares maintain precision
        assert!(shares >= SHARE_PRECISION_OFFSET, "Shares lost precision");
        assert_eq!(shares % SHARE_PRECISION_OFFSET, 0, "Shares have fractional component");
        
        // Verify raw storage
        verify_raw_storage(&dx_btc, 
            shares / SHARE_PRECISION_OFFSET,  // Raw shares
            small_deposit                     // Raw assets
        );
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_withdrawal_precision() -> Result<()> {
        // First deposit
        let deposit_amount = VIRTUAL_ASSETS;
        let context = setup_deposit_context(deposit_amount)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

        let dx_btc = DxBtc::new();
        let shares = dx_btc.deposit()?.value;
        
        // Withdraw half the shares
        let mut withdraw_context = setup_context()?;
        withdraw_context.incoming_alkanes.0.push(AlkaneTransfer {
            id: withdraw_context.myself.clone(),
            value: shares / 2,
        });
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(withdraw_context));

        let (shares_transfer, assets_transfer) = dx_btc.withdraw()?;
        
        // Verify precision in transfers
        assert_eq!(shares_transfer.value, shares / 2, "Share transfer mismatch");
        assert_eq!(assets_transfer.value, deposit_amount / 2, "Asset transfer mismatch");
        
        // Verify raw storage
        verify_raw_storage(&dx_btc, 
            VIRTUAL_SHARES / 2,     // Raw shares (half remaining)
            deposit_amount / 2      // Raw assets (half remaining)
        );
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_exchange_rate_consistency() -> Result<()> {
        let dx_btc = DxBtc::new();
        
        // Make deposits of different sizes
        let deposits = vec![MIN_DEPOSIT, MIN_DEPOSIT * 2, MIN_DEPOSIT * 3];
        let mut total_shares = 0u128;
        let mut total_assets = 0u128;
        
        for deposit_amount in deposits {
            let context = setup_deposit_context(deposit_amount)?;
            MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
            
            let shares = dx_btc.deposit()?.value;
            total_shares += shares / SHARE_PRECISION_OFFSET;
            total_assets += deposit_amount;
        }
        
        // Verify raw storage matches running totals
        verify_raw_storage(&dx_btc, total_shares, total_assets);
        
        // Verify exchange rate consistency
        let final_context = setup_deposit_context(MIN_DEPOSIT)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(final_context));
        
        let preview_shares = dx_btc.preview_deposit(MIN_DEPOSIT)?;
        let actual_shares = dx_btc.deposit()?.value;
        
        assert_eq!(preview_shares, actual_shares, "Preview/actual share mismatch");
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn test_virtual_offset_protection() -> Result<()> {
        let dx_btc = DxBtc::new();
        
        // Use minimum deposit to test virtual protection
        let small_deposit = MIN_DEPOSIT;
        let context = setup_deposit_context(small_deposit)?;
        MOCK_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
        
        let shares = dx_btc.deposit()?.value;
        
        // Even minimum deposit should get meaningful shares due to virtual offset
        assert!(shares >= SHARE_PRECISION_OFFSET, "Virtual protection failed");
        
        // Verify raw storage maintains correct values
        verify_raw_storage(&dx_btc, 
            shares / SHARE_PRECISION_OFFSET,  // Raw shares
            small_deposit                     // Raw assets
        );
        Ok(())
    }
} 
