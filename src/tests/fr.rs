use alkanes::message::AlkaneMessageContext;
use alkanes_support::cellpack::Cellpack;
use alkanes_support::id::AlkaneId;
use anyhow::Result;
use bitcoin::blockdata::transaction::OutPoint;
use metashrew_support::index_pointer::KeyValuePointer;
use protorune::{balance_sheet::load_sheet, message::MessageContext, tables::RuneTable};

use protorune_support::utils::consensus_encode;

use alkanes::indexer::index_block;
use alkanes::tests::helpers as alkane_helpers;
use crate::tests::std::fr_btc_build;
#[allow(unused_imports)]
use metashrew::{get_cache, index_pointer::IndexPointer, println, stdio::stdout};
use alkane_helpers::clear;
use std::fmt::Write;
use wasm_bindgen_test::wasm_bindgen_test;
#[wasm_bindgen_test]
fn test_genesis() -> Result<()> {
    clear();
    let block_height = 850_000;
    let cellpacks: Vec<Cellpack> = [
        //auth token factory init
        Cellpack {
            target: AlkaneId { block: 3, tx: 0 },
            inputs: vec![0],
        },
    ]
    .into();
    let test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
        [fr_btc_build::get_bytes(), vec![]].into(),
        cellpacks,
    );
    let len = test_block.txdata.len();
    let outpoint = OutPoint {
        txid: test_block.txdata[len - 1].compute_txid(),
        vout: 0
    };
    index_block(&test_block, block_height)?;
    let ptr = RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
        .OUTPOINT_TO_RUNES
        .select(&consensus_encode(&outpoint)?);
    let sheet = load_sheet(&ptr);
    println!("balances at end {:?}", sheet);
    Ok(())
}
