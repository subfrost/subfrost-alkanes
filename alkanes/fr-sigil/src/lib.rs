use alkanes_runtime::runtime::AlkaneResponder;
use alkanes_runtime::{declare_alkane, message::MessageDispatch, storage::StoragePointer};
#[allow(unused_imports)]
use alkanes_runtime::{
    println,
    stdio::{stdout, Write},
};
use alkanes_std_factory_support::MintableToken;
use alkanes_support::{context::Context, parcel::AlkaneTransfer, response::CallResponse};
use anyhow::{anyhow, Result};
use metashrew_support::compat::{to_arraybuffer_layout, to_passback_ptr};
use metashrew_support::index_pointer::KeyValuePointer;
use std::sync::Arc;

#[derive(Default)]
pub struct FrSigil(());

impl MintableToken for FrSigil {
    fn name(&self) -> String {
      String::from("frSIGIL")
    }
    fn symbol(&self) -> String {
      String::from("frSIGIL")
    }
}

#[derive(MessageDispatch)]
enum FrSigilMessage {
    #[opcode(0)]
    Initialize {
        amount: u128
    },

    #[opcode(1)]
    Authenticate,

    #[opcode(99)]
    #[returns(String)]
    GetName,

    #[opcode(100)]
    #[returns(String)]
    GetSymbol,
}

impl FrSigil {
    fn initialize(&self, amount: u128) -> Result<CallResponse> {
        self.observe_initialization()?;
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());
        response.alkanes = context.incoming_alkanes.clone();
        response.alkanes.0.push(AlkaneTransfer {
            id: context.myself.clone(),
            value: amount,
        });
        Ok(response)
    }

    fn authenticate(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());

        if context.incoming_alkanes.0.len() != 1 {
            return Err(anyhow!(
                "did not authenticate with only the authentication token"
            ));
        }
        let transfer = context.incoming_alkanes.0[0].clone();
        if transfer.id != context.myself.clone() {
            return Err(anyhow!("supplied alkane is not authentication token"));
        }
        if transfer.value < 1 {
            return Err(anyhow!(
                "less than 1 unit of authentication token supplied to authenticate"
            ));
        }
        response.data = vec![0x01];
        response.alkanes.0.push(transfer);
        Ok(response)
    }
    fn get_name(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());

        response.data = self.name().into_bytes().to_vec();
        Ok(response)
    }

    fn get_symbol(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());

        response.data = self.symbol().into_bytes().to_vec();
        Ok(response)
    }
    fn fallback(&self) -> Result<CallResponse> {
      Ok(CallResponse::forward(&self.context()?.incoming_alkanes))
    }
}

impl AlkaneResponder for FrSigil {}

// Use the new macro format
declare_alkane! {
    impl AlkaneResponder for FrSigil {
        type Message = FrSigilMessage;
    }
}
