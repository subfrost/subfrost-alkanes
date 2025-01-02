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
use alkanes_support::storage::StorageMap;
use metashrew_support::compat::{to_arraybuffer_layout, to_ptr};
use std::cell::RefCell;

#[derive(Default)]
pub struct FROST {
    // Token metadata
    name: RefCell<String>,
    symbol: RefCell<String>,
    decimals: RefCell<u8>,
    initialized: RefCell<bool>,
    
    // Token state
    total_supply: RefCell<u64>,
    balances: RefCell<StorageMap>,
    allowances: RefCell<StorageMap>,
}

impl FROST {
    fn total_supply(&self) -> u64 {
        *self.total_supply.borrow()
    }

    fn balance_of(&self, owner: &[u8]) -> u64 {
        let balances = self.balances.borrow();
        match balances.get(owner) {
            Some(balance) => u64::from_le_bytes(balance.as_slice().try_into().unwrap_or([0; 8])),
            None => 0
        }
    }

    fn allowance(&self, owner: &[u8], spender: &[u8]) -> u64 {
        let mut key = owner.to_vec();
        key.extend_from_slice(spender);
        let allowances = self.allowances.borrow();
        match allowances.get(key) {
            Some(amount) => u64::from_le_bytes(amount.as_slice().try_into().unwrap_or([0; 8])),
            None => 0
        }
    }

    fn observe_initialization(&self) -> Result<()> {
        if *self.initialized.borrow() {
            return Err(anyhow!("already initialized"));
        }
        Ok(())
    }

    fn set_data(&self, name: String, symbol: String, decimals: u8) -> Result<()> {
        self.observe_initialization()?;
        *self.name.borrow_mut() = name;
        *self.symbol.borrow_mut() = symbol;
        *self.decimals.borrow_mut() = decimals;
        *self.initialized.borrow_mut() = true;
        Ok(())
    }

    fn mint(&self, to: &[u8], amount: u64) -> Result<()> {
        let mut balances = self.balances.borrow_mut();
        let current = self.balance_of(to);
        balances.set(to.to_vec(), (current + amount).to_le_bytes().to_vec());
        *self.total_supply.borrow_mut() += amount;
        Ok(())
    }

    fn burn(&self, from: &[u8], amount: u64) -> Result<()> {
        let current = self.balance_of(from);
        if current < amount {
            return Err(anyhow!("insufficient balance"));
        }
        let mut balances = self.balances.borrow_mut();
        balances.set(from.to_vec(), (current - amount).to_le_bytes().to_vec());
        *self.total_supply.borrow_mut() -= amount;
        Ok(())
    }

    fn transfer(&self, from: &[u8], to: &[u8], amount: u64) -> Result<()> {
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(anyhow!("insufficient balance"));
        }
        
        let mut balances = self.balances.borrow_mut();
        balances.set(from.to_vec(), (from_balance - amount).to_le_bytes().to_vec());
        
        let to_balance = self.balance_of(to);
        balances.set(to.to_vec(), (to_balance + amount).to_le_bytes().to_vec());
        
        Ok(())
    }

    fn approve(&self, owner: &[u8], spender: &[u8], amount: u64) -> Result<()> {
        let mut key = owner.to_vec();
        key.extend_from_slice(spender);
        let mut allowances = self.allowances.borrow_mut();
        allowances.set(key, amount.to_le_bytes().to_vec());
        Ok(())
    }
}

impl AlkaneResponder for FROST {
    fn execute(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut inputs = context.inputs.clone();
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());
        
        match shift_or_err(&mut inputs)? {
            0 => { // Initialize
                let initial_supply: u64 = shift_or_err(&mut inputs)?.try_into().map_err(|_| anyhow!("supply overflow"))?;
                let name = String::from("FROST Token");
                let symbol = String::from("FROST");
                let decimals = 8u8;
                
                self.set_data(name, symbol, decimals)?;
                
                // Mint initial supply to sender
                let sender_id = shift_or_err(&mut inputs)?;
                let sender = sender_id.to_le_bytes().to_vec();
                self.mint(&sender, initial_supply)?;
                
                Ok(response)
            }
            1 => { // Transfer
                let amount: u64 = shift_or_err(&mut inputs)?.try_into().map_err(|_| anyhow!("amount overflow"))?;
                let sender_id = shift_or_err(&mut inputs)?;
                let recipient_id = shift_or_err(&mut inputs)?;
                
                let sender = sender_id.to_le_bytes().to_vec();
                let recipient = recipient_id.to_le_bytes().to_vec();
                
                self.transfer(&sender, &recipient, amount)?;
                Ok(response)
            }
            2 => { // Approve
                let amount: u64 = shift_or_err(&mut inputs)?.try_into().map_err(|_| anyhow!("amount overflow"))?;
                let owner_id = shift_or_err(&mut inputs)?;
                let spender_id = shift_or_err(&mut inputs)?;
                
                let owner = owner_id.to_le_bytes().to_vec();
                let spender = spender_id.to_le_bytes().to_vec();
                
                self.approve(&owner, &spender, amount)?;
                Ok(response)
            }
            3 => { // TransferFrom
                let amount: u64 = shift_or_err(&mut inputs)?.try_into().map_err(|_| anyhow!("amount overflow"))?;
                let owner_id = shift_or_err(&mut inputs)?;
                let spender_id = shift_or_err(&mut inputs)?;
                let recipient_id = shift_or_err(&mut inputs)?;
                
                let owner = owner_id.to_le_bytes().to_vec();
                let spender = spender_id.to_le_bytes().to_vec();
                let recipient = recipient_id.to_le_bytes().to_vec();
                
                let allowed = self.allowance(&owner, &spender);
                if allowed < amount {
                    return Err(anyhow!("insufficient allowance"));
                }
                
                self.transfer(&owner, &recipient, amount)?;
                
                // Update allowance
                self.approve(&owner, &spender, allowed - amount)?;
                
                Ok(response)
            }
            101 => { // View total supply
                response.data = self.total_supply().to_le_bytes().to_vec();
                Ok(response)
            }
            102 => { // View balance
                let owner_id = shift_or_err(&mut inputs)?;
                let owner = owner_id.to_le_bytes().to_vec();
                response.data = self.balance_of(&owner).to_le_bytes().to_vec();
                Ok(response)
            }
            103 => { // View allowance
                let owner_id = shift_or_err(&mut inputs)?;
                let spender_id = shift_or_err(&mut inputs)?;
                
                let owner = owner_id.to_le_bytes().to_vec();
                let spender = spender_id.to_le_bytes().to_vec();
                
                response.data = self.allowance(&owner, &spender).to_le_bytes().to_vec();
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
    let mut response = to_arraybuffer_layout(&FROST::default().run());
    to_ptr(&mut response) + 4
}
