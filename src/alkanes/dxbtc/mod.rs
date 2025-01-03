use std::cell::RefCell;
#[cfg(not(test))]
use std::io::Cursor;

use alkanes_runtime::runtime::AlkaneResponder;
use alkanes_support::context::Context;
use alkanes_support::id::AlkaneId;
use alkanes_support::parcel::AlkaneTransfer;
use alkanes_support::response::CallResponse;
use alkanes_support::storage::StorageMap;
use alkanes_support::utils::shift_or_err;
use anyhow::{anyhow, Result};

// Constants for virtual offset protection
const VIRTUAL_OFFSET: u128 = 1_000_000;  // 1M virtual offset for both assets and shares

#[cfg(test)]
thread_local! {
    static MOCK_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}

#[derive(Default)]
pub struct DxBtc {
    pub deposit_token: RefCell<Option<AlkaneId>>,
    pub total_supply: RefCell<u128>,
    pub total_deposits: RefCell<u128>,
    pub balances: RefCell<StorageMap>,
}

impl DxBtc {
    fn context(&self) -> Result<Context> {
        #[cfg(test)]
        {
            return MOCK_CONTEXT.with(|ctx| {
                ctx.borrow().clone().ok_or(anyhow!("no context set"))
            });
        }

        #[cfg(not(test))]
        {
            let mut cursor = Cursor::new(Vec::new());
            Context::parse(&mut cursor)
        }
    }

    pub fn get_key_for_alkane_id(id: &AlkaneId) -> Vec<u8> {
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
            .checked_add(VIRTUAL_OFFSET)
            .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
        
        let total_supply_with_virtual = total_supply
            .checked_add(VIRTUAL_OFFSET)
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
            .checked_add(VIRTUAL_OFFSET)
            .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
        
        let total_supply_with_virtual = total_supply
            .checked_add(VIRTUAL_OFFSET)
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
        if assets == 0 {
            return Err(anyhow!("cannot preview deposit of zero amount"));
        }
        
        // For first deposit, give 1:1 shares
        let total_supply = *self.total_supply.borrow();
        if total_supply == 0 {
            return Ok(assets);
        }
        
        self.calculate_shares(assets)
    }

    pub fn preview_withdraw(&self, shares: u128) -> Result<u128> {
        if shares == 0 {
            return Err(anyhow!("cannot preview withdrawal of zero shares"));
        }
        
        // For first withdrawal, give 1:1 assets
        let total_supply = *self.total_supply.borrow();
        if total_supply == 0 {
            return Ok(shares);
        }
        
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
            /* get_shares(owner) */
            3 => {
                let owner = shift_or_err(&mut inputs)?;
                let mut response = CallResponse::default();
                let owner_key = owner.to_le_bytes();
                response.data = self.get_shares(&owner_key).to_le_bytes().to_vec();
                Ok(response)
            },
            /* preview_deposit(assets) */
            5 => {
                let assets = shift_or_err(&mut inputs)?;
                let mut response = CallResponse::default();
                response.data = self.preview_deposit(assets)?.to_le_bytes().to_vec();
                Ok(response)
            },
            /* preview_mint(shares) */
            6 => {
                let shares = shift_or_err(&mut inputs)?;
                let mut response = CallResponse::default();
                response.data = self.preview_deposit(shares)?.to_le_bytes().to_vec();
                Ok(response)
            },
            /* preview_withdraw(shares) */
            7 => {
                let shares = shift_or_err(&mut inputs)?;
                let mut response = CallResponse::default();
                response.data = self.preview_withdraw(shares)?.to_le_bytes().to_vec();
                Ok(response)
            },
            /* preview_redeem(assets) */
            8 => {
                let assets = shift_or_err(&mut inputs)?;
                let mut response = CallResponse::default();
                response.data = self.preview_withdraw(assets)?.to_le_bytes().to_vec();
                Ok(response)
            },
            /* Any other opcode */
            _ => Ok(CallResponse::default()),
        }
    }
}

#[cfg(test)]
impl DxBtc {
    pub fn set_mock_context(context: Context) {
        MOCK_CONTEXT.with(|c| {
            *c.borrow_mut() = Some(context);
        });
    }
} 