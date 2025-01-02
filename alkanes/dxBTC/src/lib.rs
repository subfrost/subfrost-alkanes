use alkanes_runtime::runtime::AlkaneResponder;
#[allow(unused_imports)]
use alkanes_runtime::{
    println,
    stdio::{stdout, Write},
};
use anyhow::{anyhow, Result};
use alkanes_support::utils::shift_or_err;
use alkanes_support::{parcel::AlkaneTransfer, response::CallResponse};
use alkanes_support::id::AlkaneId;
use metashrew_support::compat::{to_arraybuffer_layout, to_ptr};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
pub struct DxBtc {
    deposit_token: RefCell<Option<AlkaneId>>,
    total_deposits: RefCell<u128>,
    total_supply: RefCell<u128>,
    shares: RefCell<HashMap<Vec<u8>, u128>>, // address -> shares mapping
}

impl DxBtc {
    fn calculate_mint_amount(&self, deposit_amount: u128) -> u128 {
        let total_deposits = *self.total_deposits.borrow();
        let total_supply = *self.total_supply.borrow();
        if total_deposits == 0 {
            deposit_amount
        } else {
            // Calculate based on ratio of existing deposits to total supply
            (deposit_amount * total_supply) / total_deposits
        }
    }

    fn calculate_withdrawal_amount(&self, shares_amount: u128) -> u128 {
        let total_supply = *self.total_supply.borrow();
        let total_deposits = *self.total_deposits.borrow();
        if total_supply == 0 {
            0
        } else {
            // Calculate proportional amount of deposits to withdraw
            (shares_amount * total_deposits) / total_supply
        }
    }

    fn total_supply(&self) -> u128 {
        *self.total_supply.borrow()
    }

    fn get_shares(&self, address: &[u8]) -> u128 {
        *self.shares.borrow().get(address).unwrap_or(&0)
    }

    fn deposit(&self, amount: u128, sender: Vec<u8>) -> Result<AlkaneTransfer> {
        let context = self.context()?;
        let mint_amount = self.calculate_mint_amount(amount);
        
        // Update state
        *self.total_deposits.borrow_mut() += amount;
        *self.total_supply.borrow_mut() += mint_amount;
        
        // Update shares
        let mut shares = self.shares.borrow_mut();
        *shares.entry(sender).or_default() += mint_amount;
        
        Ok(AlkaneTransfer {
            id: context.myself.clone(),
            value: mint_amount,
        })
    }

    fn withdraw(&self, shares_amount: u128, sender: Vec<u8>) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
        let context = self.context()?;
        
        // Verify user has enough shares
        let current_shares = self.get_shares(&sender);
        if current_shares < shares_amount {
            return Err(anyhow!("insufficient shares"));
        }

        // Calculate withdrawal amount
        let withdrawal_amount = self.calculate_withdrawal_amount(shares_amount);
        if withdrawal_amount == 0 {
            return Err(anyhow!("zero withdrawal amount"));
        }

        // Update state
        *self.total_supply.borrow_mut() -= shares_amount;
        *self.total_deposits.borrow_mut() -= withdrawal_amount;
        
        // Update shares
        let mut shares = self.shares.borrow_mut();
        if let Some(user_shares) = shares.get_mut(&sender) {
            *user_shares -= shares_amount;
        }

        let deposit_token = self.deposit_token.borrow()
            .as_ref()
            .ok_or_else(|| anyhow!("deposit token not initialized"))?
            .clone();

        // Return both the shares burn transfer and deposit token transfer
        Ok((
            AlkaneTransfer {
                id: context.myself.clone(),
                value: shares_amount,
            },
            AlkaneTransfer {
                id: deposit_token,
                value: withdrawal_amount,
            }
        ))
    }
}

impl AlkaneResponder for DxBtc {
    fn execute(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut inputs = context.inputs.clone();
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());
        match shift_or_err(&mut inputs)? {
            0 => {
                // Initialize with deposit token address
                let block = shift_or_err(&mut inputs)?;
                let tx = shift_or_err(&mut inputs)?;
                *self.deposit_token.borrow_mut() = Some(AlkaneId::new(block, tx));
                Ok(response)
            }
            1 => {
                // Deposit and mint
                let deposit_amount = shift_or_err(&mut inputs)?;
                let sender_id = shift_or_err(&mut inputs)?;
                let sender = sender_id.to_le_bytes().to_vec();
                let transfer = self.deposit(deposit_amount, sender)?;
                response.alkanes.0.push(transfer);
                Ok(response)
            }
            2 => {
                // Withdraw
                let shares_amount = shift_or_err(&mut inputs)?;
                let sender_id = shift_or_err(&mut inputs)?;
                let sender = sender_id.to_le_bytes().to_vec();
                let (shares_transfer, deposit_transfer) = self.withdraw(shares_amount, sender)?;
                response.alkanes.0.push(shares_transfer);
                response.alkanes.0.push(deposit_transfer);
                Ok(response)
            }
            3 => {
                // View shares
                let address_id = shift_or_err(&mut inputs)?;
                let address = address_id.to_le_bytes().to_vec();
                response.data = self.get_shares(&address).to_le_bytes().to_vec();
                Ok(response)
            }
            101 => {
                response.data = self.total_supply().to_le_bytes().to_vec();
                Ok(response)
            }
            _ => {
                Err(anyhow!("unrecognized opcode"))
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __execute() -> i32 {
    let mut response = to_arraybuffer_layout(&DxBtc::default().run());
    to_ptr(&mut response) + 4
}

