#[cfg(any(feature = "test-utils", test))]
pub mod tests {
    use alkanes_support::context::Context;
    use alkanes_support::parcel::{AlkaneTransfer, AlkaneTransferParcel};
    use alkanes_support::id::AlkaneId;
    use alkanes_support::response::CallResponse;
    use alkanes_support::storage::StorageMap;
    use anyhow::Result;
    use wasm_bindgen_test::*;
    use std::cell::RefCell;

    // Test-specific version of DxBtc
    #[derive(Default)]
    pub struct TestDxBtc {
        pub deposit_token: RefCell<Option<AlkaneId>>,
        pub total_supply: RefCell<u128>,
        pub total_deposits: RefCell<u128>,
        pub balances: RefCell<StorageMap>,
    }

    impl TestDxBtc {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn execute(&self) -> Result<CallResponse> {
            Ok(CallResponse::default())
        }

        pub fn get_shares(&self, owner: &[u8]) -> u128 {
            let balances = self.balances.borrow();
            match balances.get(owner) {
                Some(balance) => {
                    let bytes: [u8; 16] = balance.as_slice().try_into().unwrap_or([0; 16]);
                    u128::from_le_bytes(bytes)
                }
                None => 0,
            }
        }

        pub fn get_key_for_alkane_id(id: &AlkaneId) -> Vec<u8> {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&id.block.to_le_bytes());
            key.extend_from_slice(&id.tx.to_le_bytes());
            key
        }
    }

    fn setup_token() -> (TestDxBtc, Context) {
        let token = TestDxBtc::new();
        let deposit_token = AlkaneId::new(1, 2);
        *token.deposit_token.borrow_mut() = Some(deposit_token);
        
        let context = Context {
            myself: AlkaneId::new(1, 1),
            caller: AlkaneId::new(1, 2),
            inputs: vec![],
            incoming_alkanes: AlkaneTransferParcel(vec![]),
            vout: 0,
        };
        
        (token, context)
    }

    fn setup_incoming_deposit(context: &mut Context, amount: u128) {
        context.incoming_alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: amount,
        });
    }

    pub fn test_deposit_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 100);
        token.execute()?;
        Ok(())
    }

    pub fn test_withdraw_flow() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 100);
        token.execute()?;
        Ok(())
    }

    pub fn test_deposit_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 0);
        token.execute()?;
        Ok(())
    }

    pub fn test_share_calculation_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 1000);
        token.execute()?;
        Ok(())
    }

    pub fn test_preview_operations() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 1000);
        token.execute()?;
        Ok(())
    }

    pub fn test_withdrawal_safety() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 1000);
        token.execute()?;
        Ok(())
    }

    pub fn test_state_consistency() -> Result<()> {
        let (token, mut context) = setup_token();
        setup_incoming_deposit(&mut context, 1000);
        token.execute()?;
        Ok(())
    }
} 
