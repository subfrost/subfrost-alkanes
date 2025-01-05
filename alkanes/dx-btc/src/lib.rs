use alkanes_support::context::Context;
use alkanes_support::id::AlkaneId;
use alkanes_support::parcel::AlkaneTransfer;
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread_local;

thread_local! {
    pub static MOCK_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}

pub const VIRTUAL_SHARES: u128 = 1_000_000;
pub const VIRTUAL_ASSETS: u128 = 1_000_000;
pub const SHARE_PRECISION_OFFSET: u128 = 1_000_000_000;
pub const MIN_DEPOSIT: u128 = 1_000_000;

pub struct DxBtc {
    pub deposit_token: AlkaneId,
    pub total_supply: RefCell<u128>,    // Raw shares
    pub total_deposits: RefCell<u128>,   // Raw assets
    pub balances: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
}

impl DxBtc {
    pub fn new() -> Self {
        Self {
            deposit_token: AlkaneId::new(1, 2),
            total_supply: RefCell::new(0),
            total_deposits: RefCell::new(0),
            balances: RefCell::new(HashMap::new()),
        }
    }

    pub fn deposit(&self, context: &Context) -> Result<AlkaneTransfer> {
        let deposit_transfer = context
            .incoming_alkanes
            .0
            .iter()
            .find(|transfer| transfer.id == self.deposit_token)
            .ok_or_else(|| anyhow!("Deposit transfer not found"))?;

        let amount = deposit_transfer.value;
        
        if amount == 0 {
            return Err(anyhow!("Cannot deposit zero amount"));
        }
        if amount < MIN_DEPOSIT {
            return Err(anyhow!("Deposit amount {} below minimum {}", amount, MIN_DEPOSIT));
        }

        let sender_balance = self.get_balance(&deposit_transfer.id)?;
        if sender_balance < amount {
            return Err(anyhow!("Insufficient balance for deposit: has {}, needs {}", sender_balance, amount));
        }

        let shares = self.calculate_shares(amount)?;
        
        // Update balances
        self.set_balance(&deposit_transfer.id, &self.deposit_token, sender_balance.checked_sub(amount)
            .ok_or_else(|| anyhow!("Balance underflow during deposit"))?)?;
            
        let receiver_shares = self.get_shares(&context.myself)?;
        self.set_balance(&context.myself, &self.myself(), receiver_shares.checked_add(shares)
            .ok_or_else(|| anyhow!("Share overflow during deposit"))?)?;
        
        // Update total supply and deposits
        self.update_state_deposit(shares, amount)?;
        
        Ok(AlkaneTransfer {
            id: context.myself.clone(),
            value: shares,
        })
    }

    pub fn withdraw(&self, context: &Context) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
        let share_transfer = context
            .incoming_alkanes
            .0
            .iter()
            .find(|transfer| transfer.id == context.myself)
            .ok_or_else(|| anyhow!("Share transfer not found"))?;

        let shares = share_transfer.value;
        
        if shares == 0 {
            return Err(anyhow!("Cannot withdraw zero shares"));
        }

        let sender_shares = self.get_shares(&context.myself)?;
        if sender_shares < shares {
            return Err(anyhow!("Insufficient shares for withdrawal: has {}, needs {}", sender_shares, shares));
        }

        let assets = self.calculate_withdrawal_amount(shares)?;
        
        // Update balances with overflow checks
        self.set_balance(&context.myself, &self.myself(), sender_shares.checked_sub(shares)
            .ok_or_else(|| anyhow!("Share underflow during withdrawal"))?)?;
            
        let receiver_balance = self.get_balance(&context.myself)?;
        self.set_balance(&context.myself, &self.deposit_token, receiver_balance.checked_add(assets)
            .ok_or_else(|| anyhow!("Balance overflow during withdrawal"))?)?;
        
        // Update total supply and deposits
        self.update_state_withdraw(shares, assets)?;
        
        Ok((
            AlkaneTransfer {
                id: context.myself.clone(),
                value: shares,
            },
            AlkaneTransfer {
                id: self.deposit_token.clone(),
                value: assets,
            }
        ))
    }

    pub fn preview_deposit(&self, amount: u128) -> Result<u128> {
        if amount < MIN_DEPOSIT {
            return Err(anyhow!("Deposit amount {} below minimum {}", amount, MIN_DEPOSIT));
        }
        self.calculate_shares(amount)
    }

    pub fn preview_withdraw(&self, shares: u128) -> Result<u128> {
        if shares == 0 {
            return Err(anyhow!("Cannot preview withdraw zero shares"));
        }
        self.calculate_withdrawal_amount(shares)
    }

    fn calculate_shares(&self, amount: u128) -> Result<u128> {
        let total_supply = *self.total_supply.borrow();
        let total_deposits = *self.total_deposits.borrow();
        
        if total_supply == 0 {
            Ok(amount
                .checked_mul(SHARE_PRECISION_OFFSET)
                .and_then(|x| x.checked_mul(VIRTUAL_SHARES))
                .and_then(|x| x.checked_div(VIRTUAL_ASSETS))
                .ok_or_else(|| anyhow!("Share calculation overflow"))?)
        } else {
            let total_supply_with_virtual = total_supply
                .checked_add(VIRTUAL_SHARES)
                .ok_or_else(|| anyhow!("Total supply overflow"))?;

            let total_deposits_with_virtual = total_deposits
                .checked_add(VIRTUAL_ASSETS)
                .ok_or_else(|| anyhow!("Total deposits overflow"))?;

            amount
                .checked_mul(total_supply_with_virtual)
                .and_then(|x| x.checked_mul(SHARE_PRECISION_OFFSET))
                .and_then(|x| x.checked_div(total_deposits_with_virtual))
                .ok_or_else(|| anyhow!("Share calculation overflow"))
        }
    }

    fn calculate_withdrawal_amount(&self, shares: u128) -> Result<u128> {
        let total_supply = *self.total_supply.borrow();
        let total_deposits = *self.total_deposits.borrow();

        let total_supply_with_virtual = total_supply
            .checked_add(VIRTUAL_SHARES)
            .ok_or_else(|| anyhow!("Total supply overflow"))?;

        let total_deposits_with_virtual = total_deposits
            .checked_add(VIRTUAL_ASSETS)
            .ok_or_else(|| anyhow!("Total deposits overflow"))?;

        shares
            .checked_mul(total_deposits_with_virtual)
            .and_then(|x| x.checked_div(total_supply_with_virtual))
            .and_then(|x| x.checked_div(SHARE_PRECISION_OFFSET))
            .ok_or_else(|| anyhow!("Withdrawal calculation overflow"))
    }

    pub fn get_balance(&self, id: &AlkaneId) -> Result<u128> {
        let key = self.balance_key(id, &self.deposit_token);
        let balance = self.balances
            .borrow()
            .get(&key)
            .map(|bytes| self.decode_u128(bytes))
            .transpose()?
            .unwrap_or(0);
        
        Ok(balance)
    }

    pub fn get_shares(&self, owner: &AlkaneId) -> Result<u128> {
        let key = self.balance_key(owner, &self.myself());
        let shares = self.balances
            .borrow()
            .get(&key)
            .map(|bytes| self.decode_u128(bytes))
            .transpose()?
            .unwrap_or(0);
            
        Ok(shares)
    }

    pub fn set_balance(&self, id: &AlkaneId, token: &AlkaneId, amount: u128) -> Result<()> {
        let key = self.balance_key(id, token);
        let value = self.encode_u128(amount);
        self.balances.borrow_mut().insert(key, value);
        Ok(())
    }

    fn balance_key(&self, owner: &AlkaneId, token: &AlkaneId) -> Vec<u8> {
        let mut key = Vec::with_capacity(32);
        key.extend_from_slice(&owner.block.to_le_bytes());
        key.extend_from_slice(&owner.tx.to_le_bytes());
        key.extend_from_slice(&token.block.to_le_bytes());
        key.extend_from_slice(&token.tx.to_le_bytes());
        key
    }

    fn encode_u128(&self, value: u128) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn decode_u128(&self, bytes: &[u8]) -> Result<u128> {
        if bytes.len() != 16 {
            return Err(anyhow!("Invalid balance encoding: expected 16 bytes, got {}", bytes.len()));
        }
        let mut array = [0u8; 16];
        array.copy_from_slice(bytes);
        Ok(u128::from_le_bytes(array))
    }

    fn myself(&self) -> AlkaneId {
        MOCK_CONTEXT.with(|ctx| {
            ctx.borrow()
                .as_ref()
                .expect("Context not initialized")
                .myself
                .clone()
        })
    }

    fn update_state_deposit(&self, shares: u128, assets: u128) -> Result<()> {
        let new_supply = self.total_supply
            .borrow()
            .checked_add(shares / SHARE_PRECISION_OFFSET)
            .ok_or_else(|| anyhow!("Supply overflow during deposit"))?;
            
        let new_deposits = self.total_deposits
            .borrow()
            .checked_add(assets)
            .ok_or_else(|| anyhow!("Deposits overflow during deposit"))?;

        if new_deposits == 0 && new_supply > 0 {
            return Err(anyhow!("Invalid state: supply {} without deposits", new_supply));
        }

        *self.total_supply.borrow_mut() = new_supply;
        *self.total_deposits.borrow_mut() = new_deposits;
        Ok(())
    }

    fn update_state_withdraw(&self, shares: u128, assets: u128) -> Result<()> {
        let new_supply = self.total_supply
            .borrow()
            .checked_sub(shares / SHARE_PRECISION_OFFSET)
            .ok_or_else(|| anyhow!("Supply underflow during withdrawal"))?;
            
        let new_deposits = self.total_deposits
            .borrow()
            .checked_sub(assets)
            .ok_or_else(|| anyhow!("Deposits underflow during withdrawal"))?;

        if new_deposits > 0 && new_supply == 0 {
            return Err(anyhow!("Invalid state: deposits {} without supply", new_deposits));
        }

        *self.total_supply.borrow_mut() = new_supply;
        *self.total_deposits.borrow_mut() = new_deposits;
        Ok(())
    }
}
