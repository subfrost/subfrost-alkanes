pub mod alkanes {
    pub mod dxbtc {
        use alkanes_runtime::runtime::AlkaneResponder;
        use metashrew_support::compat::{to_arraybuffer_layout, to_ptr};
        use anyhow::{anyhow, Result};
        pub use alkanes_support::id::AlkaneId;
        pub use alkanes_support::response::CallResponse;
        pub use alkanes_support::storage::StorageMap;
        pub use alkanes_support::parcel::AlkaneTransfer;
        pub use alkanes_support::utils::shift_or_err;
        pub use alkanes_support::context::Context;
        use std::cell::RefCell;
        use std::io::Cursor;

        // Constants for virtual offset protection
        pub const VIRTUAL_SHARES: u128 = 1_000_000;  // 1M virtual shares
        pub const VIRTUAL_ASSETS: u128 = 1_000_000;  // 1M virtual assets

        #[derive(Default)]
        pub struct DxBtc {
            pub deposit_token: RefCell<Option<AlkaneId>>,
            pub total_supply: RefCell<u128>,
            pub total_deposits: RefCell<u128>,
            pub balances: RefCell<StorageMap>,
        }

        impl DxBtc {
            fn context(&self) -> Result<Context> {
                let mut cursor = Cursor::new(Vec::new());
                Context::parse(&mut cursor)
            }

            fn get_key_for_alkane_id(id: &AlkaneId) -> Vec<u8> {
                let mut key = Vec::with_capacity(16);
                key.extend_from_slice(&id.block.to_le_bytes());
                key.extend_from_slice(&id.tx.to_le_bytes());
                key
            }

            pub fn get_shares(&self, owner: &[u8]) -> u128 {
                let balances = self.balances.borrow();
                match balances.get(owner) {
                    Some(balance) => {
                        let bytes: [u8; 16] = balance.as_slice().try_into().unwrap_or([0; 16]);
                        u128::from_le_bytes(bytes)
                    },
                    None => 0
                }
            }

            // Calculate shares based on deposit amount and current vault state
            fn calculate_shares(&self, deposit_amount: u128) -> Result<u128> {
                let total_deposits = *self.total_deposits.borrow();
                let total_supply = *self.total_supply.borrow();
                
                // For first deposit, give 1:1 shares
                if total_supply == 0 {
                    return Ok(deposit_amount);
                }
                
                // Add virtual offsets for subsequent deposits
                let total_deposits_with_virtual = total_deposits
                    .checked_add(VIRTUAL_ASSETS)
                    .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
                
                let total_supply_with_virtual = total_supply
                    .checked_add(VIRTUAL_SHARES)
                    .ok_or_else(|| anyhow!("total_supply_with_virtual overflow"))?;
                
                // Calculate shares with virtual offset protection
                let shares = deposit_amount
                    .checked_mul(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("shares calculation overflow"))?;
                
                shares
                    .checked_div(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in shares calculation"))
            }

            // Calculate withdrawal amount based on shares
            fn calculate_withdrawal_amount(&self, shares_amount: u128) -> Result<u128> {
                let total_deposits = *self.total_deposits.borrow();
                let total_supply = *self.total_supply.borrow();
                
                // Handle edge case of no shares in circulation
                if total_supply == 0 {
                    return Ok(0);
                }
                
                // Add virtual offsets
                let total_deposits_with_virtual = total_deposits
                    .checked_add(VIRTUAL_ASSETS)
                    .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
                
                let total_supply_with_virtual = total_supply
                    .checked_add(VIRTUAL_SHARES)
                    .ok_or_else(|| anyhow!("total_supply_with_virtual overflow"))?;
                
                // Calculate withdrawal amount with virtual offset protection
                let assets = shares_amount
                    .checked_mul(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("assets calculation overflow"))?;
                
                assets
                    .checked_div(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in assets calculation"))
            }

            pub fn deposit(&self) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                
                // Verify deposit token is initialized
                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                // Find the deposit transfer
                let deposit_transfer = context.incoming_alkanes.0.iter()
                    .find(|transfer| transfer.id == deposit_token)
                    .ok_or_else(|| anyhow!("deposit transfer not found"))?;

                let amount = deposit_transfer.value;
                if amount == 0 {
                    return Err(anyhow!("cannot deposit zero amount"));
                }

                // Calculate shares
                let shares = self.calculate_shares(amount)?;
                if shares == 0 {
                    return Err(anyhow!("calculated shares amount is zero"));
                }

                // Update state
                *self.total_deposits.borrow_mut() += amount;
                *self.total_supply.borrow_mut() += shares;
                
                // Update shares using caller key
                let caller_key = Self::get_key_for_alkane_id(&context.caller);
                let current_shares = self.get_shares(&caller_key);
                let mut balances = self.balances.borrow_mut();
                let new_balance = current_shares
                    .checked_add(shares)
                    .ok_or_else(|| anyhow!("deposit would overflow user balance"))?;
                balances.set(caller_key, new_balance.to_le_bytes().to_vec());
                
                // Return share transfer
                Ok(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: shares,
                })
            }

            pub fn withdraw(&self) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
                let context = self.context()?;

                // Find the shares transfer
                let shares_transfer = context.incoming_alkanes.0.iter()
                    .find(|transfer| transfer.id == context.myself)
                    .ok_or_else(|| anyhow!("shares transfer not found"))?;

                let shares_to_burn = shares_transfer.value;
                let caller_key = Self::get_key_for_alkane_id(&context.caller);
                let current_shares = self.get_shares(&caller_key);

                if current_shares < shares_to_burn {
                    return Err(anyhow!("insufficient shares"));
                }

                let withdrawal_amount = self.calculate_withdrawal_amount(shares_to_burn)?;
                if withdrawal_amount == 0 {
                    return Err(anyhow!("calculated withdrawal amount is zero"));
                }

                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                // Update state
                *self.total_supply.borrow_mut() -= shares_to_burn;
                *self.total_deposits.borrow_mut() -= withdrawal_amount;
                
                // Update shares
                let mut balances = self.balances.borrow_mut();
                balances.set(caller_key, 
                    (current_shares - shares_to_burn).to_le_bytes().to_vec());

                // Return both transfers
                Ok((
                    AlkaneTransfer {
                        id: context.myself.clone(),
                        value: shares_to_burn,
                    },
                    AlkaneTransfer {
                        id: deposit_token,
                        value: withdrawal_amount,
                    }
                ))
            }

            pub fn preview_deposit(&self, assets: u128) -> Result<u128> {
                self.calculate_shares(assets)
            }

            pub fn preview_withdraw(&self, shares: u128) -> Result<u128> {
                self.calculate_withdrawal_amount(shares)
            }

            pub fn get_total_assets(&self) -> u128 {
                *self.total_deposits.borrow()
            }
        }

        impl AlkaneResponder for DxBtc {
            fn execute(&self) -> Result<CallResponse> {
                let context = self.context()?;
                let mut inputs = context.inputs.clone();

                match shift_or_err(&mut inputs)? {
                    /* receive() - just accept incoming alkanes */
                    0 => {
                        Ok(CallResponse::default())
                    },
                    /* deposit() */
                    1 => {
                        let mut response = CallResponse::default();
                        response.alkanes.0.push(self.deposit()?);
                        Ok(response)
                    },
                    /* withdraw() */
                    2 => {
                        let mut response = CallResponse::default();
                        let (shares_transfer, assets_transfer) = self.withdraw()?;
                        response.alkanes.0.push(shares_transfer);
                        response.alkanes.0.push(assets_transfer);
                        Ok(response)
                    },
                    /* preview_deposit(assets) */
                    3 => {
                        let assets = shift_or_err(&mut inputs)?;
                        let mut response = CallResponse::default();
                        response.data = self.preview_deposit(assets)?.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* preview_withdraw(shares) */
                    4 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let mut response = CallResponse::default();
                        response.data = self.preview_withdraw(shares)?.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* get_shares(owner) */
                    5 => {
                        let owner = shift_or_err(&mut inputs)?;
                        let mut response = CallResponse::default();
                        let owner_key = owner.to_le_bytes();
                        response.data = self.get_shares(&owner_key).to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* Any other opcode */
                    _ => Ok(CallResponse::default()),
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn __execute() -> i32 {
            let mut response = to_arraybuffer_layout(&DxBtc::default().run());
            to_ptr(&mut response) + 4
        }

        #[cfg(test)]
        mod tests {
            use super::*;
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
                let response = token.execute()?;
                assert!(response.alkanes.0.is_empty(), "Receive should return empty response");
                
                // Test deposit
                let deposit_amount = 1000;
                setup_incoming_deposit(&mut context, deposit_amount);
                
                let mut inputs = vec![1]; // deposit opcode
                context.inputs = inputs;
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
            fn test_withdraw_flow() -> Result<()> {
                let (token, mut context) = setup_token();
                
                // First deposit to have something to withdraw
                let deposit_amount = 1000;
                setup_incoming_deposit(&mut context, deposit_amount);
                token.deposit()?;
                
                // Now withdraw
                let shares_to_withdraw = 500;
                context.incoming_alkanes.0.clear();
                context.incoming_alkanes.0.push(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: shares_to_withdraw,
                });
                
                let mut inputs = vec![2]; // withdraw opcode
                context.inputs = inputs;
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
                let mut inputs = vec![3, amount]; // preview_deposit opcode
                context.inputs = inputs;
                DxBtc::set_mock_context(context.clone());
                
                let response = token.execute()?;
                let preview_shares = u128::from_le_bytes(response.data.try_into().unwrap());
                assert_eq!(preview_shares, amount, "First deposit preview should be 1:1");
                
                Ok(())
            }
        }
    }
}
