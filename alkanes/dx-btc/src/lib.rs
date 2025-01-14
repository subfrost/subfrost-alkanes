use alkanes_support::context::Context;
use alkanes_support::id::AlkaneId;
use alkanes_support::parcel::AlkaneTransfer;
use alkanes_support::utils::{shift_or_err, shift_id_or_err, overflow_error};
use alkanes_support::response::CallResponse;
use metashrew_support::compat::{to_arraybuffer_layout, to_passback_ptr};
use metashrew_support::index_pointer::KeyValuePointer;
use alkanes_runtime::{storage::{StoragePointer}, runtime::AlkaneResponder, declare_alkane, auth::AuthenticatedResponder, token::Token};
use anyhow::{Result, anyhow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::thread_local;
use std::sync::{Arc};

#[cfg(test)]
use alkanes_runtime::imports::set_mock_context;

thread_local! {
    pub static MOCK_CONTEXT: RefCell<Option<Context>> = RefCell::new(Some(Context::default()));
}

pub const VIRTUAL_SHARES: u128 = 1_000_000;
pub const VIRTUAL_ASSETS: u128 = 1_000_000;
pub const SHARE_PRECISION_OFFSET: u128 = 1_000_000_000;
pub const MIN_DEPOSIT: u128 = 1_000_000;

#[derive(Default)]
pub struct DxBtc(());

impl Token for DxBtc {
    fn name(&self) -> String {
        String::from("Subfrost DxBTC")
    }
    fn symbol(&self) -> String {
        String::from("dxBTC")
    }
}

impl AuthenticatedResponder for DxBtc {}

impl DxBtc {
    pub fn new() -> Self {
        Self(())
    }

    fn deposit_token_pointer() -> StoragePointer {
        StoragePointer::from_keyword("/deposit-token")
    }

    fn total_supply_pointer() -> StoragePointer {
        StoragePointer::from_keyword("/totalsupply")
    }

    fn total_deposits_pointer() -> StoragePointer {
        StoragePointer::from_keyword("/totaldeposits")
    }

    fn min_deposit_pointer() -> StoragePointer {
        StoragePointer::from_keyword("/min-deposit")
    }

    fn balance_pointer(owner: &AlkaneId, token: &AlkaneId) -> StoragePointer {
        let mut key = String::from("/balance/");
        key.push_str(&format!("{}.{}/{}.{}", 
            owner.block, owner.tx,
            token.block, token.tx
        ));
        StoragePointer::from_keyword(&key)
    }

    fn shares_pointer(owner: &AlkaneId) -> StoragePointer {
        let mut key = String::from("/shares/");
        key.push_str(&format!("{}.{}", owner.block, owner.tx));
        StoragePointer::from_keyword(&key)
    }

    pub fn get_deposit_token() -> AlkaneId {
        Self::deposit_token_pointer().get().as_ref().to_vec().try_into().unwrap_or_else(|_| AlkaneId::default())
    }

    pub fn total_supply(&self) -> u128 {
        Self::total_supply_pointer().get_value::<u128>()
    }

    pub fn total_deposits(&self) -> u128 {
        Self::total_deposits_pointer().get_value::<u128>()
    }

    pub fn get_min_deposit(&self) -> u128 {
        Self::min_deposit_pointer().get_value::<u128>()
    }

    fn set_deposit_token(id: AlkaneId) {
        Self::deposit_token_pointer().set(Arc::new(id.into()));
    }

    fn set_total_supply(&self, value: u128) {
        Self::total_supply_pointer().set_value::<u128>(value);
    }

    fn set_total_deposits(&self, value: u128) {
        Self::total_deposits_pointer().set_value::<u128>(value);
    }

    fn set_min_deposit(&self, value: u128) -> Result<()> {
        self.only_owner()?;
        Self::min_deposit_pointer().set_value::<u128>(value);
        Ok(())
    }

    pub fn get_balance(&self, owner: &AlkaneId, token: &AlkaneId) -> u128 {
        let pointer = Self::balance_pointer(owner, token);
        if pointer.get().len() == 0 {
            0
        } else {
            pointer.get_value::<u128>()
        }
    }

    pub fn set_balance(&self, owner: &AlkaneId, token: &AlkaneId, amount: u128) {
        Self::balance_pointer(owner, token).set_value::<u128>(amount);
    }

    fn observe_initialization(&self) -> Result<()> {
        let mut pointer = StoragePointer::from_keyword("/initialized");
        if pointer.get().len() == 0 {
            pointer.set_value::<u8>(0x01);
            Ok(())
        } else {
            Err(anyhow!("already initialized"))
        }
    }

    fn update_state_deposit(&self, shares: u128, assets: u128) -> Result<()> {
        let new_supply = overflow_error(self.total_supply()
            .checked_add(shares / SHARE_PRECISION_OFFSET))?;
            
        let new_deposits = overflow_error(self.total_deposits()
            .checked_add(assets))?;

        if new_deposits == 0 && new_supply > 0 {
            return Err(anyhow!("Invalid state: supply {} without deposits", new_supply));
        }

        self.set_total_supply(new_supply);
        self.set_total_deposits(new_deposits);
        Ok(())
    }

    fn update_state_withdraw(&self, shares: u128, assets: u128) -> Result<()> {
        let new_supply = overflow_error(self.total_supply()
            .checked_sub(shares / SHARE_PRECISION_OFFSET))?;
            
        let new_deposits = overflow_error(self.total_deposits()
            .checked_sub(assets))?;

        if new_deposits > 0 && new_supply == 0 {
            return Err(anyhow!("Invalid state: deposits {} without supply", new_deposits));
        }

        self.set_total_supply(new_supply);
        self.set_total_deposits(new_deposits);
        Ok(())
    }

    pub fn deposit(&self, context: &Context) -> Result<AlkaneTransfer> {
        let deposit_token = Self::get_deposit_token();
        let deposit_transfer = context
            .incoming_alkanes
            .0
            .iter()
            .find(|transfer| transfer.id == deposit_token)
            .ok_or_else(|| anyhow!("Deposit transfer not found"))?;

        let amount = deposit_transfer.value;
        
        if amount == 0 {
            return Err(anyhow!("Cannot deposit zero amount"));
        }
        if amount < self.get_min_deposit() {
            return Err(anyhow!("Deposit amount {} below minimum {}", amount, self.get_min_deposit()));
        }

        let sender_balance = self.get_balance(&deposit_transfer.id, &deposit_token);
        if sender_balance < amount {
            return Err(anyhow!("Insufficient balance for deposit: has {}, needs {}", sender_balance, amount));
        }

        let shares = self.calculate_shares(amount)?;
        
        // Update balances
        self.set_balance(&deposit_transfer.id, &deposit_token, 
            overflow_error(sender_balance.checked_sub(amount))?);
            
        let receiver_shares = self.get_shares(&context.myself);
        self.set_shares(&context.myself, 
            overflow_error(receiver_shares.checked_add(shares))?);
        
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

        let sender_shares = self.get_shares(&context.myself);
        if sender_shares < shares {
            return Err(anyhow!("Insufficient shares for withdrawal: has {}, needs {}", sender_shares, shares));
        }

        let assets = self.calculate_withdrawal_amount(shares)?;
        let deposit_token = Self::get_deposit_token();
        
        // Update balances with overflow checks
        self.set_shares(&context.myself, 
            overflow_error(sender_shares.checked_sub(shares))?);
            
        let receiver_balance = self.get_balance(&context.myself, &deposit_token);
        self.set_balance(&context.myself, &deposit_token, 
            overflow_error(receiver_balance.checked_add(assets))?);
        
        // Update total supply and deposits
        self.update_state_withdraw(shares, assets)?;
        
        Ok((
            AlkaneTransfer {
                id: context.myself.clone(),
                value: shares,
            },
            AlkaneTransfer {
                id: deposit_token,
                value: assets,
            }
        ))
    }

    fn preview_deposit(&self, amount: u128) -> Result<u128> {
        if amount < self.get_min_deposit() {
            return Err(anyhow!("Deposit amount {} below minimum {}", amount, self.get_min_deposit()));
        }
        self.calculate_shares(amount)
    }

    fn preview_withdraw(&self, shares: u128) -> Result<u128> {
        if shares == 0 {
            return Err(anyhow!("Cannot preview withdraw zero shares"));
        }
        self.calculate_withdrawal_amount(shares)
    }

    fn calculate_shares(&self, amount: u128) -> Result<u128> {
        let total_supply = self.total_supply();
        let total_deposits = self.total_deposits();
        
        if total_supply == 0 {
            Ok(amount
                .checked_mul(SHARE_PRECISION_OFFSET)
                .and_then(|x| x.checked_mul(VIRTUAL_SHARES))
                .and_then(|x| x.checked_div(VIRTUAL_ASSETS))
                .ok_or_else(|| anyhow!("Share calculation overflow"))?)
        } else {
            let total_supply_with_virtual = overflow_error(total_supply
                .checked_add(VIRTUAL_SHARES))?;

            let total_deposits_with_virtual = overflow_error(total_deposits
                .checked_add(VIRTUAL_ASSETS))?;

            overflow_error(amount
                .checked_mul(total_supply_with_virtual)
                .and_then(|x| x.checked_mul(SHARE_PRECISION_OFFSET))
                .and_then(|x| x.checked_div(total_deposits_with_virtual)))
        }
    }

    fn calculate_withdrawal_amount(&self, shares: u128) -> Result<u128> {
        let total_supply = self.total_supply();
        let total_deposits = self.total_deposits();

        let total_supply_with_virtual = overflow_error(total_supply
            .checked_add(VIRTUAL_SHARES))?;

        let total_deposits_with_virtual = overflow_error(total_deposits
            .checked_add(VIRTUAL_ASSETS))?;

        overflow_error(shares
            .checked_mul(total_deposits_with_virtual)
            .and_then(|x| x.checked_div(total_supply_with_virtual))
            .and_then(|x| x.checked_div(SHARE_PRECISION_OFFSET)))
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

    pub fn get_shares(&self, owner: &AlkaneId) -> u128 {
        let pointer = Self::shares_pointer(owner);
        if pointer.get().len() == 0 {
            0
        } else {
            pointer.get_value::<u128>()
        }
    }

    fn set_shares(&self, owner: &AlkaneId, amount: u128) {
        Self::shares_pointer(owner).set_value::<u128>(amount);
    }
}

impl AlkaneResponder for DxBtc {
    fn execute(&self) -> Result<CallResponse> {
        #[cfg(test)]
        set_mock_context(Context::default());
        let context = self.context()?;
        let mut inputs = context.inputs.clone();
        let mut response = CallResponse::forward(&context.incoming_alkanes);

        match shift_or_err(&mut inputs)? {
            0 => {
                self.observe_initialization()?;
                Self::set_deposit_token(shift_id_or_err(&mut inputs)?);
                Ok(response)
            }
            1 => {
                // Deposit
                let transfer = self.deposit(&context)?;
                response.alkanes.0.push(transfer);
                Ok(response)
            }
            2 => {
                // Withdraw
                let (share_transfer, asset_transfer) = self.withdraw(&context)?;
                response.alkanes.0.push(share_transfer);
                response.alkanes.0.push(asset_transfer);
                Ok(response)
            }
            3 => {
                // Preview deposit
                let amount = shift_or_err(&mut inputs)?;
                let shares = self.preview_deposit(amount)?;
                response.data = shares.to_le_bytes().to_vec();
                Ok(response)
            }
            4 => {
                // Preview withdraw
                let shares = shift_or_err(&mut inputs)?;
                let assets = self.preview_withdraw(shares)?;
                response.data = assets.to_le_bytes().to_vec();
                Ok(response)
            }
            99 => {
                response.data = self.name().into_bytes().to_vec();
                Ok(response)
            }
            100 => {
                response.data = self.symbol().into_bytes().to_vec();
                Ok(response)
            }
            101 => {
                response.data = self.total_supply().to_le_bytes().to_vec();
                Ok(response)
            }
            102 => {
                response.data = self.total_deposits().to_le_bytes().to_vec();
                Ok(response)
            }
            103 => {
                // Get balance
                let owner = shift_id_or_err(&mut inputs)?;
                let token = shift_id_or_err(&mut inputs)?;
                response.data = self.get_balance(&owner, &token).to_le_bytes().to_vec();
                Ok(response)
            }
            104 => {
                // Set min deposit (admin)
                self.only_owner()?;
                self.set_min_deposit(shift_or_err(&mut inputs)?)?;
                Ok(response)
            }
            105 => {
                // Get min deposit
                response.data = self.get_min_deposit().to_le_bytes().to_vec();
                Ok(response)
            }
            _ => {
                Err(anyhow!("opcode not supported"))
            }
        }
    }
}

declare_alkane!{DxBtc}
