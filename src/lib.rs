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

            pub fn deposit(&self, amount: u64, sender: Vec<u8>) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                let mint_amount = amount; // 1:1 ratio for simplicity
                
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
