use alkanes_support::id::AlkaneId;
use alkanes_support::context::Context;
use alkanes_support::parcel::AlkaneTransferParcel;

pub fn create_test_context(myself: AlkaneId, caller: AlkaneId) -> Context {
    Context {
        myself,
        caller,
        inputs: vec![],
        incoming_alkanes: AlkaneTransferParcel(vec![]),
        vout: 0,
    }
}

pub fn create_test_alkane_id(chain_id: u64, token_id: u64) -> AlkaneId {
    AlkaneId::new(chain_id.into(), token_id.into())
} 