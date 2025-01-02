pub mod precompiled {
    pub mod fr_btc_build;
}

pub mod alkanes {
    pub use crate::precompiled::fr_btc_build::get_bytes as fr_btc_build;
    pub mod dxbtc {
        use alkanes_runtime::runtime::AlkaneResponder;
        use anyhow::{anyhow, Result};
        pub use alkanes_support::id::AlkaneId;
        pub use alkanes_support::response::CallResponse;
        pub use alkanes_support::storage::StorageMap;
        pub use alkanes_support::parcel::AlkaneTransfer;
        pub use alkanes_support::utils::shift_or_err;
        pub use alkanes_support::context::Context;
        use std::cell::RefCell;

        thread_local! {
            static MOCK_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
        }

        // Constants for virtual offset protection and precision
        const VIRTUAL_SHARES: u64 = 1_000_000;  // 1M virtual shares
        const VIRTUAL_ASSETS: u64 = 1_000_000;  // 1M virtual assets
        const DECIMALS_MULTIPLIER: u128 = 1_000_000_000;  // 9 decimals for precision

        #[derive(Default)]
        pub struct DxBtc {
            pub deposit_token: RefCell<Option<AlkaneId>>,
            pub total_supply: RefCell<u64>,
            pub total_deposits: RefCell<u64>,
            pub balances: RefCell<StorageMap>,
        }

        impl DxBtc {
            pub fn get_shares(&self, owner: &[u8]) -> u64 {
                let balances = self.balances.borrow();
                match balances.get(owner) {
                    Some(balance) => u64::from_le_bytes(balance.as_slice().try_into().unwrap_or([0; 8])),
                    None => 0
                }
            }

            // Calculate shares based on deposit amount and current vault state
            fn calculate_shares(&self, deposit_amount: u64) -> u64 {
                let total_supply = *self.total_supply.borrow();
                let total_deposits = *self.total_deposits.borrow();

                if total_supply == 0 || total_deposits == 0 {
                    // First deposit gets 1:1 shares
                    deposit_amount
                } else {
                    // Calculate shares based on the proportion of the total vault value
                    // shares = deposit_amount * total_supply / total_deposits
                    ((deposit_amount as u128 * total_supply as u128) / total_deposits as u128) as u64
                }
            }

            // Calculate withdrawal amount based on shares
            fn calculate_withdrawal_amount(&self, shares_amount: u64) -> u64 {
                let total_supply = *self.total_supply.borrow();
                let total_deposits = *self.total_deposits.borrow();

                if total_supply == 0 || total_deposits == 0 {
                    shares_amount
                } else {
                    // withdrawal_amount = shares * total_deposits / total_supply
                    ((shares_amount as u128 * total_deposits as u128) / total_supply as u128) as u64
                }
            }

            pub fn deposit(&self, amount: u64, sender: Vec<u8>) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                
                // Calculate shares for the deposit amount
                let mint_amount = self.calculate_shares(amount);
                
                // Get current shares first
                let current_shares = {
                    let balances = self.balances.borrow();
                    match balances.get(&sender) {
                        Some(balance) => u64::from_le_bytes(balance.as_slice().try_into().unwrap_or([0; 8])),
                        None => 0
                    }
                };
                
                // Update state
                *self.total_deposits.borrow_mut() += amount;
                *self.total_supply.borrow_mut() += mint_amount;
                
                // Update shares
                let mut balances = self.balances.borrow_mut();
                balances.set(sender, (current_shares + mint_amount).to_le_bytes().to_vec());
                
                Ok(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: mint_amount as u128,
                })
            }

            pub fn withdraw(&self, shares_amount: u64, sender: Vec<u8>) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
                let context = self.context()?;
                
                // Get current shares first
                let current_shares = {
                    let balances = self.balances.borrow();
                    match balances.get(&sender) {
                        Some(balance) => u64::from_le_bytes(balance.as_slice().try_into().unwrap_or([0; 8])),
                        None => 0
                    }
                };

                // Verify user has enough shares
                if current_shares < shares_amount {
                    return Err(anyhow!("insufficient shares"));
                }

                // Calculate withdrawal amount based on shares
                let withdrawal_amount = self.calculate_withdrawal_amount(shares_amount);
                if withdrawal_amount == 0 {
                    return Err(anyhow!("zero withdrawal amount"));
                }

                // Get deposit token
                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                // Update state
                *self.total_supply.borrow_mut() -= shares_amount;
                *self.total_deposits.borrow_mut() -= withdrawal_amount;
                
                // Update shares
                let mut balances = self.balances.borrow_mut();
                balances.set(sender, (current_shares - shares_amount).to_le_bytes().to_vec());

                Ok((
                    AlkaneTransfer {
                        id: context.myself.clone(),
                        value: shares_amount as u128,
                    },
                    AlkaneTransfer {
                        id: deposit_token,
                        value: withdrawal_amount as u128,
                    }
                ))
            }

            pub fn context(&self) -> Result<Context> {
                MOCK_CONTEXT.with(|ctx| {
                    ctx.borrow().clone().ok_or(anyhow!("no context set"))
                })
            }

            pub fn set_mock_context(context: Context) {
                MOCK_CONTEXT.with(|ctx| {
                    *ctx.borrow_mut() = Some(context);
                });
            }

            pub fn preview_deposit(&self, assets: u64) -> u128 {
                let total_deposits = u128::from(*self.total_deposits.borrow()) + u128::from(VIRTUAL_ASSETS);
                let total_supply = u128::from(*self.total_supply.borrow()) + u128::from(VIRTUAL_SHARES);

                if total_deposits == u128::from(VIRTUAL_ASSETS) {
                    // First real deposit after virtual offset
                    u128::from(assets)
                } else {
                    // Calculate shares with high precision and virtual offset protection
                    (u128::from(assets) * DECIMALS_MULTIPLIER * total_supply) / 
                    (total_deposits * DECIMALS_MULTIPLIER)
                }
            }

            pub fn preview_withdraw(&self, shares: u64) -> u128 {
                let total_supply = u128::from(*self.total_supply.borrow()) + u128::from(VIRTUAL_SHARES);
                let total_deposits = u128::from(*self.total_deposits.borrow()) + u128::from(VIRTUAL_ASSETS);

                if total_supply == u128::from(VIRTUAL_SHARES) {
                    0
                } else {
                    // Calculate withdrawal amount with high precision and virtual offset protection
                    (u128::from(shares) * DECIMALS_MULTIPLIER * total_deposits) / 
                    (total_supply * DECIMALS_MULTIPLIER)
                }
            }
        }

        impl AlkaneResponder for DxBtc {
            fn execute(&self) -> Result<CallResponse> {
                let context = self.context()?;
                let mut inputs = context.inputs.clone();
                let response = CallResponse::forward(&context.incoming_alkanes);
                match shift_or_err(&mut inputs)? {
                    0 => {
                        let mut deposit_token = self.deposit_token.borrow_mut();
                        *deposit_token = Some(AlkaneId::new(1, 2));
                        Ok(response)
                    }
                    _ => Err(anyhow!("unrecognized opcode")),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    pub mod dxbtc_tests;
}
