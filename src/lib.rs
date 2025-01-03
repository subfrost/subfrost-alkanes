#[cfg(any(feature = "test-utils", test))]
pub mod tests;

#[cfg(test)]
mod test {
    use crate::tests::dxbtc;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_dxbtc_deposit_flow() {
        dxbtc::tests::test_deposit_flow().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_withdraw_flow() {
        dxbtc::tests::test_withdraw_flow().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_deposit_safety() {
        dxbtc::tests::test_deposit_safety().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_share_calculation_safety() {
        dxbtc::tests::test_share_calculation_safety().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_preview_operations() {
        dxbtc::tests::test_preview_operations().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_withdrawal_safety() {
        dxbtc::tests::test_withdrawal_safety().unwrap();
    }

    #[wasm_bindgen_test]
    fn test_dxbtc_state_consistency() {
        dxbtc::tests::test_state_consistency().unwrap();
    }
}
