use alkanes_runtime::{auth::AuthenticatedResponder, token::Token};
#[allow(unused_imports)]
use alkanes_runtime::{
    println,
    stdio::{stdout, Write},
};
use alkanes_runtime::{runtime::AlkaneResponder, storage::StoragePointer};
use alkanes_support::utils::{shift, shift_id};
use alkanes_support::{context::Context, parcel::AlkaneTransfer, response::CallResponse};
use metashrew_support::compat::{to_arraybuffer_layout, to_passback_ptr};
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;

#[derive(Default)]
pub struct OwnedToken(());

pub trait MintableToken: Token {
    fn mint(&self, context: &Context, value: u128) -> AlkaneTransfer {
        AlkaneTransfer {
            id: context.myself.clone(),
            value,
        }
    }
}

impl Token for OwnedToken {
    fn name(&self) -> String {
        String::from("bUSD")
    }
    fn symbol(&self) -> String {
        String::from("bUSD")
    }
}
impl MintableToken for OwnedToken {}

impl AuthenticatedResponder for OwnedToken {}

impl AlkaneResponder for OwnedToken {
    fn execute(&self) -> CallResponse {
        let context = self.context().unwrap();
        let mut inputs = context.inputs.clone();
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());
        match shift(&mut inputs).unwrap() {
            /* initialize(u128, u128, u128[2]) */
            0 => {
                let mut pointer = StoragePointer::from_keyword("/initialized");
                if pointer.get().len() == 0 {
                    let auth_token_units = shift(&mut inputs).unwrap();
                    let token_units = shift(&mut inputs).unwrap();
                    let peg = shift_id(&mut inputs).unwrap();
                    response
                        .alkanes
                        .0
                        .push(self.deploy_auth_token(auth_token_units).unwrap());
                    response.alkanes.0.push(AlkaneTransfer {
                        id: context.myself.clone(),
                        value: token_units,
                    });
                    StoragePointer::from_keyword("/burn-from").set(Arc::<Vec<u8>>::new(peg.into()));
                    pointer.set(Arc::new(vec![0x01]));
                    response
                } else {
                    panic!("already initialized");
                }
            }
            /* mint_from() */
            47 => {
                let position = context.incoming_alkanes.0.iter().position(|v| v.id == StoragePointer::from_keyword("/burn-from").get().as_ref().clone().try_into().unwrap() ).unwrap();
                response.alkanes.0[position].id = context.myself.clone();
                response

            }
            /* mint(u128) */
            77 => {
                self.only_owner().unwrap();
                let token_units = shift(&mut inputs).unwrap();
                let transfer = self.mint(&context, token_units);
                response.alkanes.0.push(transfer);
                response
            }
            /* name() */
            99 => {
                response.data = self.name().into_bytes().to_vec();
                response
            }
            /* symbol() */
            100 => {
                response.data = self.symbol().into_bytes().to_vec();
                response
            }
            _ => {
                panic!("unrecognized opcode");
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn __execute() -> i32 {
    let mut response = to_arraybuffer_layout(&OwnedToken::default().run());
    to_passback_ptr(&mut response)
}
