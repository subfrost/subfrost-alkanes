use alkanes::message::AlkaneMessageContext;
use alkanes_support::id::AlkaneId;
use anyhow::Result;
use bitcoin::blockdata::transaction::OutPoint;
use bitcoin::address::{NetworkChecked};
use bitcoin::{Witness, Sequence, Amount, ScriptBuf, Address, TxIn, TxOut, Transaction};
use protorune_support::protostone::Protostone;
use protorune::protostone::Protostones;
use metashrew_support::index_pointer::KeyValuePointer;
use protorune::{test_helpers::{get_address}, balance_sheet::load_sheet, message::MessageContext, tables::RuneTable};
use protorune_support::utils::consensus_encode;
use alkanes::indexer::index_block;
use ordinals::Runestone;
use alkanes::tests::helpers as alkane_helpers;
use alkanes::precompiled::{alkanes_std_auth_token_build};
use alkanes_support::{cellpack::Cellpack, constants::AUTH_TOKEN_FACTORY_ID};
use crate::precompiled::fr_btc_build;
use alkane_helpers::{clear};
use metashrew::{println, stdio::{stdout}};
use std::fmt::Write;
use wasm_bindgen_test::wasm_bindgen_test;

pub const TEST_MULTISIG: &'static str = "bcrt1pys2f8u8yx7nu08txn9kzrstrmlmpvfprdazz9se5qr5rgtuz8htsaz3chd";
pub const ADDRESS1: &'static str = "bcrt1qzr9vhs60g6qlmk7x3dd7g3ja30wyts48sxuemv";

fn pay_to_musig(inputs: Vec<OutPoint>, amount: u64) -> Transaction {
  let protostone: Protostone = Protostone {
    burn: None,
    edicts: vec![],
    pointer: Some(1),
    refund: Some(2),
    from: None,
    protocol_tag: AlkaneMessageContext::protocol_tag(),
    message: (Cellpack {
      target: AlkaneId {
        block: 4,
        tx: 0
      },
      inputs: vec![77]
    }).encipher(),
  };
  let runestone: ScriptBuf = (Runestone {
    etching: None,
    pointer: Some(0), // points to the OP_RETURN, so therefore targets the protoburn
    edicts: Vec::new(),
    mint: None,
    protocol: vec![protostone].encipher().ok(),
  }).encipher();
  let op_return = TxOut {
    value: Amount::from_sat(0),
    script_pubkey: runestone,
  };
  let address: Address<NetworkChecked> = get_address(ADDRESS1);
  let _script_pubkey = address.script_pubkey();
  let mut tx = Transaction {
    version: bitcoin::blockdata::transaction::Version::ONE,
    lock_time: bitcoin::absolute::LockTime::ZERO,
    input: inputs.into_iter().map(|v| TxIn {
      previous_output: v,
      witness: Witness::new(),
      script_sig: ScriptBuf::new(),
      sequence: Sequence::MAX
    }).collect::<Vec<TxIn>>(),
    output: vec![
      TxOut {
        value: Amount::from_sat(amount),
        script_pubkey: get_address(TEST_MULTISIG).script_pubkey()
      },
      TxOut {
        value: Amount::from_sat(546),
        script_pubkey: _script_pubkey.clone()
      },
      op_return
    ]
  };
  
  // Add witness data
  if !tx.input.is_empty() {
    tx.input[0].witness = Witness::from_slice(&[vec![0; 32]]);
  }
  
  tx
}

fn set_signer(inputs: Vec<OutPoint>) -> Transaction {
  let protostone: Protostone = Protostone {
    burn: None,
    edicts: vec![],
    pointer: Some(1),
    refund: Some(2),
    from: None,
    protocol_tag: AlkaneMessageContext::protocol_tag(),
    message: (Cellpack {
      target: AlkaneId {
        block: 4,
        tx: 0
      },
      inputs: vec![1, 0]
    }).encipher(),
  };
  let runestone: ScriptBuf = (Runestone {
    etching: None,
    pointer: Some(0), // points to the OP_RETURN, so therefore targets the protoburn
    edicts: Vec::new(),
    mint: None,
    protocol: vec![protostone].encipher().ok(),
  }).encipher();
  let op_return = TxOut {
    value: Amount::from_sat(0),
    script_pubkey: runestone,
  };
  let address: Address<NetworkChecked> = get_address(ADDRESS1);
  let _script_pubkey = address.script_pubkey();
  let mut tx = Transaction {
    version: bitcoin::blockdata::transaction::Version::ONE,
    lock_time: bitcoin::absolute::LockTime::ZERO,
    input: inputs.into_iter().map(|v| TxIn {
      previous_output: v,
      witness: Witness::new(),
      script_sig: ScriptBuf::new(),
      sequence: Sequence::MAX
    }).collect::<Vec<TxIn>>(),
    output: vec![
      TxOut {
        value: Amount::from_sat(546),
        script_pubkey: get_address(TEST_MULTISIG).script_pubkey()
      },
      TxOut {
        value: Amount::from_sat(546),
        script_pubkey: get_address(ADDRESS1).script_pubkey()
      },
      op_return
    ]
  };
  
  // Add witness data
  if !tx.input.is_empty() {
    tx.input[0].witness = Witness::from_slice(&[vec![0; 32]]);
  }
  
  tx
}

#[wasm_bindgen_test]
fn test_synthetic_init() -> Result<()> {
    clear();
    let mut block_height = 850_000;
    let cellpacks: Vec<Cellpack> = [
        //auth token factory init
        Cellpack {
            target: AlkaneId { block: 3, tx: AUTH_TOKEN_FACTORY_ID },
            inputs: vec![100]
        },
        Cellpack {
            target: AlkaneId { block: 3, tx: 0 },
            inputs: vec![0, 1],
        }
    ]
    .into();
    
    // Get the binary data for the cellpacks
    let binary_data = fr_btc_build::get_bytes();
    
    let mut test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
        [alkanes_std_auth_token_build::get_bytes(), binary_data.clone()].into(),
        cellpacks,
    );
    
    // Add witness data to the test block transactions
    for tx in test_block.txdata.iter_mut() {
        for input in tx.input.iter_mut() {
            input.witness = Witness::from_slice(&[binary_data.clone()]);
        }
    }
    
    let len = test_block.txdata.len();
    let outpoint = OutPoint {
        txid: test_block.txdata[len - 1].compute_txid(),
        vout: 0
    };
    index_block(&test_block, block_height)?;
    let ptr = RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
        .OUTPOINT_TO_RUNES
        .select(&consensus_encode(&outpoint)?);
    
    // Load and verify initial balance sheet
    let initial_sheet = load_sheet(&ptr);
    println!("initial balances {:?}", initial_sheet);
    
    test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(vec![], vec![]);
    test_block.txdata.push(set_signer(vec![outpoint.clone()]));
    test_block.txdata.push(pay_to_musig(vec![], 500_000_000));
    block_height = block_height + 1;
    
    let len2 = test_block.txdata.len();
    let outpoint2 = OutPoint {
        txid: test_block.txdata[len2 - 1].compute_txid(),
        vout: 1
    };
    index_block(&test_block, block_height)?;
    let ptr = RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
        .OUTPOINT_TO_RUNES
        .select(&consensus_encode(&outpoint2)?);
    
    let final_sheet = load_sheet(&ptr);
    println!("balances at end {:?}", final_sheet);
    
    Ok(())
}
