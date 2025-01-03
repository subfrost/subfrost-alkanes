use alkanes_support::id::AlkaneId;
use anyhow::Result;
use std::default::Default;
use wasm_bindgen_test::*;

pub trait AlkaneTest: Default {
    fn get_deposit_token(&self) -> AlkaneId;
    fn set_mock_context(context: Vec<u8>);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alkanes::dxbtc::DxBtc;
    use super::super::utils;

    fn setup_token() -> DxBtc {
        let token = DxBtc::default();
        let deposit_token = AlkaneId::new(1u128, 2u128);
        *token.deposit_token.borrow_mut() = Some(deposit_token);
        token
    }

    #[wasm_bindgen_test]
    fn test_alkane_base_functionality() -> Result<()> {
        let token = setup_token();
        let deposit_token = token.get_deposit_token();
        let id_bytes: Vec<u8> = deposit_token.into();
        assert!(!id_bytes.is_empty());

        // Mock context setup
        let context = utils::create_test_context(
            AlkaneId::new(1u128, 1u128),
            AlkaneId::new(1u128, 3u128)
        );
        DxBtc::set_mock_context(context.clone());

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_alkane_transfer_validation() -> Result<()> {
        let token = setup_token();
        let deposit_token = token.get_deposit_token();
        let id_bytes: Vec<u8> = deposit_token.into();
        assert!(!id_bytes.is_empty());

        // Mock context setup
        let context = utils::create_test_context(
            AlkaneId::new(1u128, 1u128),
            AlkaneId::new(1u128, 3u128)
        );
        DxBtc::set_mock_context(context.clone());

        Ok(())
    }
} 