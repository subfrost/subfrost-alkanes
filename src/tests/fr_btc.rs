use crate::tests::std::fr_btc_build;
use alkanes::message::AlkaneMessageContext;
use alkanes::precompiled::alkanes_std_auth_token_build;
use alkanes::view;
use alkanes_support::constants::AUTH_TOKEN_FACTORY_ID;
use alkanes_support::gz::compress;
use alkanes_support::id::AlkaneId;
use alkanes_support::trace::Trace;
use anyhow::Result;
use bitcoin::address::NetworkChecked;
use bitcoin::blockdata::transaction::OutPoint;
use bitcoin::key::TapTweak;
use bitcoin::transaction::Version;
use bitcoin::{
    secp256k1::{self, Secp256k1, XOnlyPublicKey},
    Address, Amount, Block, Script, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
};
#[allow(unused_imports)]
use hex;
use metashrew_support::index_pointer::KeyValuePointer;
#[allow(unused_imports)]
use metashrew_support::utils::format_key;
use protorune::message::MessageContextParcel;
use protorune::protostone::Protostones;
use protorune::test_helpers::create_block_with_coinbase_tx;
use protorune::{
    balance_sheet::load_sheet, message::MessageContext, tables::RuneTable,
    test_helpers::get_address,
};
use protorune_support::balance_sheet::{BalanceSheetOperations, ProtoruneRuneId};
use protorune_support::protostone::Protostone;

use protorune_support::utils::consensus_encode;

use alkane_helpers::clear;
use alkanes::indexer::index_block;
use alkanes::network::set_view_mode;
use alkanes::tests::helpers::{self as alkane_helpers, get_last_outpoint_sheet};
use alkanes_support::cellpack::Cellpack;
#[allow(unused_imports)]
use metashrew_core::{get_cache, index_pointer::IndexPointer, println, stdio::stdout};
use ordinals::{Artifact, Runestone};
use std::fmt::Write;
use wasm_bindgen_test::wasm_bindgen_test;

fn setup_fr_btc() -> Result<Block> {
    let block_height = 880_000;
    let cellpacks: Vec<Cellpack> = [
        //auth token factory init
        Cellpack {
            target: AlkaneId {
                block: 3,
                tx: AUTH_TOKEN_FACTORY_ID,
            },
            inputs: vec![100],
        },
        Cellpack {
            target: AlkaneId { block: 3, tx: 0 },
            inputs: vec![0, 1],
        },
    ]
    .into();
    let test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
        [
            alkanes_std_auth_token_build::get_bytes(),
            fr_btc_build::get_bytes(),
        ]
        .into(),
        cellpacks,
    );
    index_block(&test_block, block_height)?;
    let sheet = get_last_outpoint_sheet(&test_block)?;
    let auth_token = ProtoruneRuneId { block: 2, tx: 1 };
    assert_eq!(sheet.get(&auth_token), 5);
    Ok(test_block)
}

pub fn create_alkane_tx_frbtc_signer_script(
    cellpacks: Vec<Cellpack>,
    previous_output: OutPoint,
) -> Transaction {
    let txins = vec![TxIn {
        previous_output,
        script_sig: ScriptBuf::new(),
        sequence: Sequence::MAX,
        witness: Witness::default(),
    }];
    let protocol_id = 1;
    let protostones: Vec<Protostone> = [cellpacks
        .into_iter()
        .map(|cellpack| Protostone {
            message: cellpack.encipher(),
            pointer: Some(0),
            refund: Some(0),
            edicts: vec![],
            from: None,
            burn: None,
            protocol_tag: protocol_id as u128,
        })
        .collect::<Vec<Protostone>>()]
    .concat();
    let runestone: ScriptBuf = (Runestone {
        etching: None,
        pointer: Some(0),
        edicts: Vec::new(),
        mint: None,
        protocol: protostones.encipher().ok(),
    })
    .encipher();

    //     // op return is at output 1
    let op_return = TxOut {
        value: Amount::from_sat(0),
        script_pubkey: runestone,
    };

    // Get the signer pubkey from the contract
    let signer_pubkey_bytes = [
        0x07, 0x9a, 0x54, 0xd0, 0xae, 0xf2, 0xb3, 0x43, 0xaa, 0xc8, 0x9c, 0x0f, 0xd7, 0x89, 0xaa,
        0xb4, 0xac, 0xb9, 0x1f, 0x00, 0xca, 0xa0, 0xf8, 0xd5, 0x15, 0x01, 0x45, 0x2c, 0xe4, 0x7c,
        0xc9, 0x7d,
    ]
    .to_vec();
    let signer_pubkey = XOnlyPublicKey::from_slice(&signer_pubkey_bytes).unwrap();
    let secp = Secp256k1::new();
    let (tweaked_signer_pubkey, _) = signer_pubkey.tap_tweak(&secp, None);
    let signer_script = ScriptBuf::new_p2tr_tweaked(tweaked_signer_pubkey);

    let txout = TxOut {
        value: Amount::from_sat(100_000_000),
        script_pubkey: signer_script,
    };
    Transaction {
        version: Version::ONE,
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: txins,
        output: vec![txout, op_return],
    }
}

fn wrap_btc() -> Result<()> {
    let fr_btc_id = AlkaneId { block: 4, tx: 0 };
    let mut block = create_block_with_coinbase_tx(880_001);
    let funding_outpoint = OutPoint {
        txid: block.txdata[0].compute_txid(),
        vout: 0,
    };
    let wrap_tx = create_alkane_tx_frbtc_signer_script(
        vec![Cellpack {
            target: fr_btc_id.clone(),
            inputs: vec![77],
        }],
        funding_outpoint,
    );

    // Create a block and index it
    block.txdata.push(wrap_tx.clone());
    index_block(&block, 880_001)?;

    let sheet = get_last_outpoint_sheet(&block)?;
    let balance = sheet.get(&fr_btc_id.clone().into());

    assert_eq!(balance, 99500000);

    Ok(())
}

#[wasm_bindgen_test]
fn test_fr_btc() -> Result<()> {
    clear();
    setup_fr_btc()?;
    Ok(())
}

#[wasm_bindgen_test]
fn test_fr_btc_wrap_correct_signer() -> Result<()> {
    clear();
    setup_fr_btc()?;
    wrap_btc()
}

// #[wasm_bindgen_test]
// fn test_fr_btc_unwrap() -> Result<()> {
//     clear();
//     let init_block = setup_fr_btc()?;
//     wrap_btc()?;

// }

#[wasm_bindgen_test]
fn test_fr_btc_wrap_incorrect_signer() -> Result<()> {
    clear();
    setup_fr_btc()?;
    let fr_btc_id = AlkaneId { block: 4, tx: 0 };
    let mut block = create_block_with_coinbase_tx(880_001);
    let funding_outpoint = OutPoint {
        txid: block.txdata[0].compute_txid(),
        vout: 0,
    };
    let wrap_tx = alkane_helpers::create_multiple_cellpack_with_witness_and_in(
        Witness::default(),
        vec![Cellpack {
            target: fr_btc_id.clone(),
            inputs: vec![77],
        }],
        funding_outpoint,
        false,
    );

    // Create a block and index it
    block.txdata.push(wrap_tx.clone());
    index_block(&block, 880_001)?;

    let sheet = get_last_outpoint_sheet(&block)?;
    let balance = sheet.get(&fr_btc_id.clone().into());

    // No BTC sent to correct signer, so no frBTC should be minted.
    assert_eq!(balance, 0);

    Ok(())
}
