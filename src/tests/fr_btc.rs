use alkanes::message::AlkaneMessageContext;
use alkanes::view;
use alkanes::precompiled::{alkanes_std_auth_token_build};
use crate::tests::std::fr_btc_build;
use alkanes_support::gz::compress;
use alkanes_support::id::AlkaneId;
use alkanes_support::constants::{AUTH_TOKEN_FACTORY_ID};
use alkanes_support::trace::Trace;
use anyhow::Result;
use bitcoin::address::NetworkChecked;
use bitcoin::blockdata::transaction::OutPoint;
use bitcoin::{Address, Amount, Script, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};
#[allow(unused_imports)]
use hex;
use metashrew_support::index_pointer::KeyValuePointer;
#[allow(unused_imports)]
use metashrew_support::utils::format_key;
use protorune::message::MessageContextParcel;
use protorune::protostone::Protostones;
use protorune::{
    balance_sheet::load_sheet, message::MessageContext, tables::RuneTable,
    test_helpers::get_address,
};
use protorune_support::protostone::Protostone;

use protorune_support::utils::consensus_encode;

use alkane_helpers::clear;
use alkanes::indexer::index_block;
use alkanes::network::set_view_mode;
use alkanes::tests::helpers as alkane_helpers;
use alkanes_support::cellpack::Cellpack;
#[allow(unused_imports)]
use metashrew_core::{get_cache, index_pointer::IndexPointer, println, stdio::stdout};
use ordinals::{Artifact, Runestone};
use std::fmt::Write;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn test_fr_btc() -> Result<()> {
    clear();
    set_view_mode();
    let mut block_height = 850_000;
    let cellpacks: Vec<Cellpack> = [
        //auth token factory init
        Cellpack {
            target: AlkaneId {
                block: 3,
                tx: AUTH_TOKEN_FACTORY_ID
            },
            inputs: vec![100],
        },
        Cellpack {
            target: AlkaneId {
                block: 3,
                tx: 0
            },
            inputs: vec![0, 1]
        }
    ]
    .into();
    let mut test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
        [alkanes_std_auth_token_build::get_bytes(), fr_btc_build::get_bytes()].into(),
        cellpacks,
    );
    let mut output: &mut TxOut  = {
      let len = test_block.txdata.len();
      let mut tx = &mut test_block.txdata[len - 1];
      let out_len = tx.output.len();
      &mut tx.output[out_len - 1]
    };
    let len = test_block.txdata.len();
    let outpoint = OutPoint {
        txid: test_block.txdata[len - 1].compute_txid(),
        vout: 0,
    };
    index_block(&test_block, block_height)?;
    let ptr = RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
        .OUTPOINT_TO_RUNES
        .select(&consensus_encode(&outpoint)?);
    let sheet = load_sheet(&ptr);
    println!("balances at end {:?}", sheet);
    println!(
        "{:?}",
        <Vec<u8> as TryInto<Trace>>::try_into(view::trace(&OutPoint {
            txid: test_block.txdata[len - 1].compute_txid(),
            vout: 3
        })?)?
    );
    let mut parcel = MessageContextParcel::default();
    parcel.height = 850001;
    parcel.calldata = (Cellpack {
        target: AlkaneId { block: 4, tx: 0 },
        inputs: vec![100001u128],
    })
    .encipher();
    println!("calldata: {:?}", &parcel.calldata);
/*
    let callresponse_data = view::simulate_parcel(&parcel, u64::MAX)?.0.data;
    println!("callresponse: {:?}", callresponse_data);
*/
    Ok(())
}
