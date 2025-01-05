use alkanes_support::context::Context;
use alkanes_support::id::AlkaneId;
use alkanes_support::parcel::AlkaneTransfer;
use alkanes_support::storage::StorageMap;
use anyhow::{Result, bail};
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
    pub total_supply: RefCell<u128>,
    pub total_deposits: RefCell<u128>,
    pub balances: RefCell<HashMap<Vec<u8>, Vec<u8>>>,
}

impl DxBtc {
    pub fn new() -> Self {
        Self {
            deposit_token: AlkaneId::new(1, 2), // BTC token
            total_supply: RefCell::new(0),
            total_deposits: RefCell::new(0),
            balances: RefCell::new(HashMap::new()),
        }
    }

    pub fn deposit(&self) -> Result<AlkaneTransfer> {
        let context = MOCK_CONTEXT.with(|ctx| ctx.borrow().clone().unwrap());
        let deposit = context.incoming_alkanes.0.first().unwrap();
        
        if deposit.value < MIN_DEPOSIT {
            bail!("Deposit amount below minimum");
        }

        let shares = self.calculate_shares(deposit.value)?;
        *self.total_supply.borrow_mut() += shares;
        *self.total_deposits.borrow_mut() += deposit.value;
        
        Ok(AlkaneTransfer {
            id: context.myself,
            value: shares,
        })
    }

    pub fn withdraw(&self) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
        let context = MOCK_CONTEXT.with(|ctx| ctx.borrow().clone().unwrap());
        let shares = context.incoming_alkanes.0.first().unwrap();
        
        let assets = self.calculate_withdrawal_amount(shares.value)?;
        
        *self.total_supply.borrow_mut() -= shares.value;
        *self.total_deposits.borrow_mut() -= assets;
        
        Ok((
            AlkaneTransfer {
                id: context.myself,
                value: shares.value,
            },
            AlkaneTransfer {
                id: self.deposit_token,
                value: assets,
            }
        ))
    }

    pub fn preview_deposit(&self, amount: u128) -> Result<u128> {
        if amount < MIN_DEPOSIT {
            bail!("Deposit amount below minimum");
        }
        self.calculate_shares(amount)
    }

    pub fn preview_withdraw(&self, shares: u128) -> Result<u128> {
        self.calculate_withdrawal_amount(shares)
    }

    pub fn calculate_shares(&self, amount: u128) -> Result<u128> {
        let total_supply = *self.total_supply.borrow();
        let total_deposits = *self.total_deposits.borrow();
        
        let total_supply_with_virtual = total_supply + VIRTUAL_SHARES * SHARE_PRECISION_OFFSET;
        let total_deposits_with_virtual = total_deposits + VIRTUAL_ASSETS;
        
        if total_supply == 0 {
            // First deposit gets shares based on virtual ratio
            Ok(amount * SHARE_PRECISION_OFFSET / 2)
        } else {
            Ok(amount * total_supply_with_virtual / total_deposits_with_virtual)
        }
    }

    pub fn calculate_withdrawal_amount(&self, shares: u128) -> Result<u128> {
        let total_supply = *self.total_supply.borrow();
        let total_deposits = *self.total_deposits.borrow();
        
        let total_supply_with_virtual = total_supply + VIRTUAL_SHARES * SHARE_PRECISION_OFFSET;
        let total_deposits_with_virtual = total_deposits + VIRTUAL_ASSETS;
        
        Ok(shares * total_deposits_with_virtual / total_supply_with_virtual)
    }
}
