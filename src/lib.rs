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
        use std::sync::Mutex;
        use once_cell::sync::Lazy;

        static MOCK_CONTEXT: Lazy<Mutex<Option<Context>>> = Lazy::new(|| Mutex::new(None));

        #[derive(Default)]
        pub struct DxBtc {
            pub deposit_token: Mutex<Option<AlkaneId>>,
            pub total_supply: Mutex<u64>,
            pub total_deposits: Mutex<u64>,
            pub balances: Mutex<StorageMap>,
        }

        impl DxBtc {
            pub fn get_shares(&self, owner: &[u8]) -> u64 {
                let balances = self.balances.lock().unwrap();
                match balances.get(owner) {
                    Some(balance) => u64::from_le_bytes(balance.as_slice().try_into().unwrap_or([0; 8])),
                    None => 0
                }
            }

            pub fn deposit(&self, amount: u64, sender: Vec<u8>) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                let mint_amount = amount; // 1:1 ratio for simplicity
                
                // Update state
                *self.total_deposits.lock().unwrap() += amount;
                *self.total_supply.lock().unwrap() += mint_amount;
                
                // Update shares
                let mut balances = self.balances.lock().unwrap();
                let current = self.get_shares(&sender);
                balances.set(sender, (current + mint_amount).to_le_bytes().to_vec());
                
                Ok(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: mint_amount as u128,
                })
            }

            pub fn withdraw(&self, shares_amount: u64, sender: Vec<u8>) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
                let context = self.context()?;
                
                // Verify user has enough shares
                let current_shares = self.get_shares(&sender);
                if current_shares < shares_amount {
                    return Err(anyhow!("insufficient shares"));
                }

                // Calculate withdrawal amount (1:1 ratio for simplicity)
                let withdrawal_amount = shares_amount;
                if withdrawal_amount == 0 {
                    return Err(anyhow!("zero withdrawal amount"));
                }

                // Update state
                *self.total_supply.lock().unwrap() -= shares_amount;
                *self.total_deposits.lock().unwrap() -= withdrawal_amount;
                
                // Update shares
                let mut balances = self.balances.lock().unwrap();
                let current = self.get_shares(&sender);
                balances.set(sender, (current - shares_amount).to_le_bytes().to_vec());

                let deposit_token = self.deposit_token.lock().unwrap()
                    .as_ref()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?
                    .clone();

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

            #[cfg(test)]
            pub fn set_mock_context(context: Context) {
                *MOCK_CONTEXT.lock().unwrap() = Some(context);
            }

            #[cfg(test)]
            pub fn clear_mock_context() {
                *MOCK_CONTEXT.lock().unwrap() = None;
            }
        }

        impl AlkaneResponder for DxBtc {
            fn context(&self) -> Result<Context> {
                #[cfg(test)]
                {
                    MOCK_CONTEXT.lock().unwrap().clone().ok_or_else(|| anyhow!("mock context not set"))
                }

                #[cfg(not(test))]
                {
                    extern "C" {
                        fn __request_context();
                        fn __load_context() -> Context;
                    }
                    unsafe {
                        __request_context();
                        Ok(__load_context())
                    }
                }
            }

            fn execute(&self) -> Result<CallResponse> {
                let context = self.context()?;
                let mut inputs = context.inputs.clone();
                let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());
                
                match shift_or_err(&mut inputs)? {
                    0 => { // Initialize with deposit token
                        let block = shift_or_err(&mut inputs)?;
                        let tx = shift_or_err(&mut inputs)?;
                        *self.deposit_token.lock().unwrap() = Some(AlkaneId::new(block, tx));
                        Ok(response)
                    }
                    1 => { // Deposit and mint
                        let deposit_amount = shift_or_err(&mut inputs)?;
                        let sender_id = shift_or_err(&mut inputs)?;
                        let sender = sender_id.to_le_bytes().to_vec();
                        let transfer = self.deposit(deposit_amount.try_into()?, sender)?;
                        response.alkanes.0.push(transfer);
                        Ok(response)
                    }
                    2 => { // Withdraw
                        let shares_amount = shift_or_err(&mut inputs)?;
                        let sender_id = shift_or_err(&mut inputs)?;
                        let sender = sender_id.to_le_bytes().to_vec();
                        let (shares_transfer, deposit_transfer) = self.withdraw(shares_amount.try_into()?, sender)?;
                        response.alkanes.0.push(shares_transfer);
                        response.alkanes.0.push(deposit_transfer);
                        Ok(response)
                    }
                    _ => {
                        Err(anyhow!("unrecognized opcode"))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    pub mod dxbtc_tests;
}
