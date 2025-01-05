#[cfg(test)]
mod tests {
    use dx_btc::{DxBtc, SHARE_PRECISION_OFFSET, VIRTUAL_SHARES, VIRTUAL_ASSETS};
    use alkanes_support::context::Context;
    use alkanes_support::id::AlkaneId;
    use alkanes_support::parcel::{AlkaneTransfer, AlkaneTransferParcel};
    use anyhow::Result;
    use wasm_bindgen_test::*;

    fn setup_token() -> (DxBtc, Context) {
        let token = DxBtc::new();
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
        let (vault, mut context) = setup_token();
        let deposit_amount = 1000;

        // First deposit should get shares with precision offset
        let shares = vault.preview_deposit(deposit_amount)?;
        assert_eq!(shares, deposit_amount * SHARE_PRECISION_OFFSET);

        // Setup and execute deposit
        setup_incoming_deposit(&mut context, deposit_amount);
        let deposit_result = vault.deposit()?;
        assert_eq!(deposit_result.value, shares);

        assert_eq!(*vault.total_deposits.borrow(), deposit_amount);
        assert_eq!(*vault.total_supply.borrow(), shares);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_withdraw_flow() -> Result<()> {
        let (vault, mut context) = setup_token();
        let deposit_amount = 1000;

        // First deposit
        setup_incoming_deposit(&mut context, deposit_amount);
        let deposit_result = vault.deposit()?;
        let shares = deposit_result.value;

        // Preview withdrawal
        let preview_amount = vault.preview_withdraw(shares)?;
        assert_eq!(preview_amount, deposit_amount);

        // Setup withdrawal
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares,
        });
        DxBtc::set_mock_context(context.clone());

        // Actual withdrawal
        let (shares_transfer, assets_transfer) = vault.withdraw()?;
        assert_eq!(shares_transfer.value, shares);
        assert_eq!(assets_transfer.value, deposit_amount);

        assert_eq!(*vault.total_deposits.borrow(), 0);
        assert_eq!(*vault.total_supply.borrow(), 0);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_preview_operations() -> Result<()> {
        let (vault, mut context) = setup_token();
        let deposit_amount = 1000;

        // Preview deposit
        let shares = vault.preview_deposit(deposit_amount)?;
        assert_eq!(shares, deposit_amount * SHARE_PRECISION_OFFSET);

        // Actual deposit
        setup_incoming_deposit(&mut context, deposit_amount);
        let deposit_result = vault.deposit()?;
        assert_eq!(deposit_result.value, shares);

        // Preview withdrawal
        let preview_amount = vault.preview_withdraw(shares)?;
        assert_eq!(preview_amount, deposit_amount);

        // Setup withdrawal
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares,
        });
        DxBtc::set_mock_context(context.clone());

        // Actual withdrawal
        let (shares_transfer, assets_transfer) = vault.withdraw()?;
        assert_eq!(shares_transfer.value, shares);
        assert_eq!(assets_transfer.value, deposit_amount);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_share_calculation_safety() -> Result<()> {
        let (vault, mut context) = setup_token();
        let first_deposit = 1000;
        let second_deposit = 500;

        // First deposit
        setup_incoming_deposit(&mut context, first_deposit);
        let first_shares = vault.deposit()?.value;
        assert_eq!(first_shares, first_deposit * SHARE_PRECISION_OFFSET);

        // Second deposit
        setup_incoming_deposit(&mut context, second_deposit);
        let second_shares = vault.deposit()?.value;
        
        // Calculate expected shares for second deposit
        let total_deposits_with_virtual = first_deposit + VIRTUAL_ASSETS;
        let total_supply_with_virtual = first_shares + VIRTUAL_SHARES;
        let expected_second_shares = second_deposit
            .checked_mul(total_supply_with_virtual)
            .unwrap()
            .checked_div(total_deposits_with_virtual)
            .unwrap();
        
        assert_eq!(second_shares, expected_second_shares);

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_withdrawal_safety() -> Result<()> {
        let (vault, mut context) = setup_token();
        let deposit_amount = 1000;

        // First deposit
        setup_incoming_deposit(&mut context, deposit_amount);
        let shares = vault.deposit()?.value;

        // Setup withdrawal
        context.incoming_alkanes.0.clear();
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares,
        });
        DxBtc::set_mock_context(context.clone());

        // Withdrawal
        let (shares_transfer, assets_transfer) = vault.withdraw()?;
        assert_eq!(shares_transfer.value, shares);
        assert_eq!(assets_transfer.value, deposit_amount);

        Ok(())
    }
} 
