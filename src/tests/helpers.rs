use bitcoin::address::{Address, NetworkChecked};
use std::str::FromStr;
use alkanes_support::context::Context;
use alkanes_support::id::AlkaneId;

pub fn get_test_address(name: &str) -> Address<NetworkChecked> {
    match name {
        "musig" => Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").expect("Valid address").assume_checked(),
        "user" => Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").expect("Valid address").assume_checked(),
        "signer" => Address::from_str("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4").expect("Valid address").assume_checked(),
        _ => panic!("Unknown test address name: {}", name)
    }
}

pub fn clear_test_environment() {
    // For now, we'll leave this empty as we're focusing on the basic test functionality
}

// Trait for common test functionality
pub trait AlkaneTest {
    fn get_deposit_token(&self) -> AlkaneId;
    fn set_mock_context(context: Context);
}

// Helper function to create a test context
pub fn create_test_context(myself: AlkaneId, caller: AlkaneId) -> Context {
    Context {
        myself,
        caller,
        incoming_alkanes: Default::default(),
        inputs: vec![],
        vout: Default::default(),
        // Add other Context fields as needed with default values
    }
} 