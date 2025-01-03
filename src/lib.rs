#[cfg(any(feature = "test-utils", test))]
pub mod tests;

#[cfg(test)]
mod test {
    use wasm_bindgen_test::*;
}
