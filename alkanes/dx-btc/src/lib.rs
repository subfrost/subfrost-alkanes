

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

            pub fn deposit(&self, _amount: u128, sender: Vec<u8>) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                // Calculate shares for the deposit amount
                let shares = self.calculate_shares(_amount)?;
                if shares == 0 {
                    return Err(anyhow!("calculated shares amount is zero"));
                }

                // Update state
                *self.total_deposits.borrow_mut() += _amount;
                *self.total_supply.borrow_mut() += shares;
                
                // Update shares
                let current_shares = self.get_shares(&sender);
                let mut balances = self.balances.borrow_mut();
                let new_balance = current_shares
                    .checked_add(shares)
                    .ok_or_else(|| anyhow!("deposit would overflow user balance"))?;
                balances.set(sender, new_balance.to_le_bytes().to_vec());
                
                Ok(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: shares,
                })
            }

            pub fn withdraw(&self, sender: Vec<u8>) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
                let context = self.context()?;

                // Verify incoming shares
                let incoming = &context.incoming_alkanes.0;
                if incoming.is_empty() {
                    return Err(anyhow!("no incoming shares found"));
                }

                // Find the shares transfer
                let shares_transfer = incoming.iter()
                    .find(|transfer| transfer.id == context.myself)
                    .ok_or_else(|| anyhow!("shares transfer not found"))?;

                // Use the actual incoming shares amount
                let shares_to_burn = shares_transfer.value;
                
                // Get current shares first
                let current_shares = self.get_shares(&sender);

                // Verify user has enough shares
                if current_shares < shares_to_burn {
                    return Err(anyhow!("insufficient shares"));
                }

                // Calculate withdrawal amount based on shares
                let withdrawal_amount = self.calculate_withdrawal_amount(shares_to_burn)?;
                if withdrawal_amount == 0 {
                    return Err(anyhow!("calculated withdrawal amount is zero"));
                }

                // Get deposit token
                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                // Update state
                *self.total_supply.borrow_mut() -= shares_to_burn;
                *self.total_deposits.borrow_mut() -= withdrawal_amount;
                
                // Update shares
                let mut balances = self.balances.borrow_mut();
                balances.set(sender, (current_shares - shares_to_burn).to_le_bytes().to_vec());

                Ok((
                    AlkaneTransfer {
                        id: context.myself.clone(),
                        value: shares_transfer.value,
                    },
                    AlkaneTransfer {
                        id: deposit_token,
                        value: withdrawal_amount,
                    }
                ))
            }

            pub fn preview_deposit(&self, assets: u128) -> Result<u128> {
                // Convert to u128 for safe calculations
                let total_deposits = *self.total_deposits.borrow();
                let total_supply = *self.total_supply.borrow();

                // For first deposit, give 1:1 shares
                if total_supply == 0 {
                    return Ok(assets);
                }

                // Add virtual offsets for subsequent deposits
                let total_deposits_with_virtual = total_deposits
                    .checked_add(VIRTUAL_ASSETS)
                    .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
                
                let total_supply_with_virtual = total_supply
                    .checked_add(VIRTUAL_SHARES)
                    .ok_or_else(|| anyhow!("total_supply_with_virtual overflow"))?;

                // Calculate shares with virtual offset protection
                let shares = assets
                    .checked_mul(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("shares calculation overflow"))?;
                
                shares
                    .checked_div(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in shares calculation"))
            }

            pub fn preview_withdraw(&self, shares: u128) -> Result<u128> {
                // Convert to u128 for safe calculations
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
                let assets = shares
                    .checked_mul(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("assets calculation overflow"))?;
                
                assets
                    .checked_div(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in assets calculation"))
            }

            pub fn convert_to_shares(&self, assets: u128) -> Result<u128> {
                let total_deposits = *self.total_deposits.borrow();
                let total_supply = *self.total_supply.borrow();

                // For first deposit, give 1:1 shares
                if total_supply == 0 {
                    return Ok(assets);
                }

                // Add virtual offsets for subsequent deposits
                let total_deposits_with_virtual = total_deposits
                    .checked_add(VIRTUAL_ASSETS)
                    .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;
                let total_supply_with_virtual = total_supply
                    .checked_add(VIRTUAL_SHARES)
                    .ok_or_else(|| anyhow!("total_supply_with_virtual overflow"))?;

                // Calculate shares with virtual offset protection
                let shares = assets
                    .checked_mul(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("shares calculation overflow"))?;
                let shares = shares
                    .checked_div(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in shares calculation"))?;
                
                // Remove virtual shares from the result
                if shares <= VIRTUAL_SHARES {
                    Ok(1) // Minimum share amount
                } else {
                    let final_shares = shares
                        .checked_sub(VIRTUAL_SHARES)
                        .ok_or_else(|| anyhow!("final shares calculation overflow"))?;
                    Ok(final_shares)
                }
            }

            pub fn convert_to_assets(&self, shares: u128) -> Result<u128> {
                let total_supply = *self.total_supply.borrow();
                let total_deposits = *self.total_deposits.borrow();

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

                // Calculate assets with virtual offset protection
                let assets = shares
                    .checked_mul(total_deposits_with_virtual)
                    .ok_or_else(|| anyhow!("assets calculation overflow"))?;
                let assets = assets
                    .checked_div(total_supply_with_virtual)
                    .ok_or_else(|| anyhow!("division by zero in assets calculation"))?;
                
                // Remove virtual assets from the result
                if assets <= VIRTUAL_ASSETS {
                    Ok(1) // Minimum withdrawal amount
                } else {
                    let final_assets = assets
                        .checked_sub(VIRTUAL_ASSETS)
                        .ok_or_else(|| anyhow!("final assets calculation overflow"))?;
                    Ok(final_assets)
                }
            }

            // Maximum amount of assets that can be deposited
            pub fn max_deposit(&self, _user: &[u8]) -> u128 {
                u128::MAX
            }

            // Maximum amount of shares that can be minted
            pub fn max_mint(&self, _user: &[u8]) -> u128 {
                u128::MAX
            }

            // Maximum amount of shares that can be withdrawn
            pub fn max_withdraw(&self, user: &[u8]) -> u128 {
                self.get_shares(user)
            }

            // Maximum amount of shares that can be redeemed
            pub fn max_redeem(&self, user: &[u8]) -> u128 {
                self.get_shares(user)
            }

            // Preview mint (similar to preview_deposit but for exact shares)
            pub fn preview_mint(&self, shares: u128) -> Result<u128> {
                let total_supply = (*self.total_supply.borrow())
                    .checked_add(VIRTUAL_SHARES)
                    .ok_or_else(|| anyhow!("total_supply_with_virtual overflow"))?;
                let total_deposits = (*self.total_deposits.borrow())
                    .checked_add(VIRTUAL_ASSETS)
                    .ok_or_else(|| anyhow!("total_deposits_with_virtual overflow"))?;

                if total_supply <= VIRTUAL_SHARES {
                    Ok(shares)
                } else {
                    let shares_u128 = shares
                        .checked_mul(total_deposits)
                        .ok_or_else(|| anyhow!("shares calculation overflow"))?;
                    shares_u128
                        .checked_div(total_supply)
                        .ok_or_else(|| anyhow!("division by zero in shares calculation"))
                }
            }

            // Preview redeem (similar to preview_withdraw but with different rounding)
            pub fn preview_redeem(&self, shares: u128) -> Result<u128> {
                self.preview_withdraw(shares)
            }

            // Get total assets managed by the vault
            pub fn total_assets(&self) -> u128 {
                *self.total_deposits.borrow()
            }

            // Mint exact shares, requiring a specific amount of assets
            pub fn mint(&self, shares: u128, sender: Vec<u8>) -> Result<AlkaneTransfer> {
                let context = self.context()?;
                
                let required_assets = self.preview_mint(shares)?;
                if required_assets == 0 {
                    return Err(anyhow!("cannot mint zero shares"));
                }
                
                let current_shares = self.get_shares(&sender);
                
                *self.total_deposits.borrow_mut() += required_assets;
                *self.total_supply.borrow_mut() += shares;
                
                let mut balances = self.balances.borrow_mut();
                let new_balance = current_shares
                    .checked_add(shares)
                    .ok_or_else(|| anyhow!("mint would overflow user balance"))?;
                balances.set(sender, new_balance.to_le_bytes().to_vec());
                
                Ok(AlkaneTransfer {
                    id: context.myself.clone(),
                    value: shares,
                })
            }

            // Redeem shares for assets
            pub fn redeem(&self, shares: u128, sender: Vec<u8>) -> Result<(AlkaneTransfer, AlkaneTransfer)> {
                if shares == 0 {
                    return Err(anyhow!("cannot redeem zero shares"));
                }
                let context = self.context()?;
                
                let assets_to_return = self.preview_redeem(shares)?;
                if assets_to_return == 0 {
                    return Err(anyhow!("cannot redeem zero assets"));
                }
                
                let current_shares = self.get_shares(&sender);
                if current_shares < shares {
                    return Err(anyhow!("insufficient shares"));
                }

                let deposit_token = self.deposit_token.borrow()
                    .clone()
                    .ok_or_else(|| anyhow!("deposit token not initialized"))?;

                *self.total_supply.borrow_mut() -= shares;
                *self.total_deposits.borrow_mut() -= assets_to_return;
                
                let mut balances = self.balances.borrow_mut();
                balances.set(sender, (current_shares - shares).to_le_bytes().to_vec());

                Ok((
                    AlkaneTransfer {
                        id: context.myself.clone(),
                        value: shares,
                    },
                    AlkaneTransfer {
                        id: deposit_token,
                        value: assets_to_return,
                    }
                ))
            }
        }

        impl AlkaneResponder for DxBtc {
            fn execute(&self) -> Result<CallResponse> {
                let context = self.context()?;
                let mut inputs = context.inputs.clone();

                let mut response = CallResponse::forward(&context.incoming_alkanes);
                match shift_or_err(&mut inputs)? {
                    /* initialize(deposit_token_id) */
                    0 => {
                        let mut deposit_token = self.deposit_token.borrow_mut();
                        *deposit_token = Some(AlkaneId::new(1, 2));
                        Ok(response)
                    },
                    /* deposit(amount, sender) */
                    1 => {
                        let amount = shift_or_err(&mut inputs)?;
                        let sender = shift_or_err(&mut inputs)?;
                        let sender_bytes = sender.to_le_bytes();
                        response.alkanes.0.push(self.deposit(amount, sender_bytes.to_vec())?);
                        Ok(response)
                    },
                    /* withdraw(sender) */
                    2 => {
                        let sender = shift_or_err(&mut inputs)?;
                        let sender_bytes = sender.to_le_bytes();
                        let (shares_transfer, assets_transfer) = self.withdraw(sender_bytes.to_vec())?;
                        response.alkanes.0.push(shares_transfer);
                        response.alkanes.0.push(assets_transfer);
                        Ok(response)
                    },
                    /* preview_deposit(assets) */
                    3 => {
                        let assets = shift_or_err(&mut inputs)?;
                        let shares = self.preview_deposit(assets)?;
                        response.data = shares.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* preview_withdraw(shares) */
                    4 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let assets = self.preview_withdraw(shares)?;
                        response.data = assets.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* convert_to_shares(assets) */
                    5 => {
                        let assets = shift_or_err(&mut inputs)?;
                        let shares = self.convert_to_shares(assets)?;
                        response.data = shares.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* convert_to_assets(shares) */
                    6 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let assets = self.convert_to_assets(shares)?;
                        response.data = assets.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* max_deposit(user) */
                    7 => {
                        let user = shift_or_err(&mut inputs)?;
                        let user_bytes = user.to_le_bytes();
                        let max = self.max_deposit(&user_bytes);
                        response.data = max.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* max_mint(user) */
                    8 => {
                        let user = shift_or_err(&mut inputs)?;
                        let user_bytes = user.to_le_bytes();
                        let max = self.max_mint(&user_bytes);
                        response.data = max.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* max_withdraw(user) */
                    9 => {
                        let user = shift_or_err(&mut inputs)?;
                        let user_bytes = user.to_le_bytes();
                        let max = self.max_withdraw(&user_bytes);
                        response.data = max.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* max_redeem(user) */
                    10 => {
                        let user = shift_or_err(&mut inputs)?;
                        let user_bytes = user.to_le_bytes();
                        let max = self.max_redeem(&user_bytes);
                        response.data = max.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* preview_mint(shares) */
                    11 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let assets = self.preview_mint(shares)?;
                        response.data = assets.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* preview_redeem(shares) */
                    12 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let assets = self.preview_redeem(shares)?;
                        response.data = assets.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* total_assets() */
                    13 => {
                        let total = self.total_assets();
                        response.data = total.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    /* mint(shares, sender) */
                    14 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let sender = shift_or_err(&mut inputs)?;
                        let sender_bytes = sender.to_le_bytes();
                        response.alkanes.0.push(self.mint(shares, sender_bytes.to_vec())?);
                        Ok(response)
                    },
                    /* redeem(shares, sender) */
                    15 => {
                        let shares = shift_or_err(&mut inputs)?;
                        let sender = shift_or_err(&mut inputs)?;
                        let sender_bytes = sender.to_le_bytes();
                        let (shares_transfer, assets_transfer) = self.redeem(shares, sender_bytes.to_vec())?;
                        response.alkanes.0.push(shares_transfer);
                        response.alkanes.0.push(assets_transfer);
                        Ok(response)
                    },
                    /* get_shares(owner) */
                    16 => {
                        let owner = shift_or_err(&mut inputs)?;
                        let owner_bytes = owner.to_le_bytes();
                        let shares = self.get_shares(&owner_bytes);
                        response.data = shares.to_le_bytes().to_vec();
                        Ok(response)
                    },
                    _ => Err(anyhow!("unrecognized opcode")),
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn __execute() -> i32 {
            let mut response = to_arraybuffer_layout(&DxBtc::default().run());
            to_ptr(&mut response) + 4
        }
    }
}
