use anyhow::Result;
use bitcoin::{OutPoint, TxOut, Amount, ScriptBuf, Txid};
use bitcoin::hashes::{Hash, sha256d};
use frbtc_support::{Payment, deserialize_payments};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn test_payment_serialization() -> Result<()> {
    let dummy_txid = Txid::from_raw_hash(sha256d::Hash::hash(&[0; 32]));
    let payment = Payment {
        spendable: OutPoint::new(dummy_txid, 0),
        output: TxOut {
            value: Amount::from_sat(1000),
            script_pubkey: ScriptBuf::new()
        }
    };

    // Test serialization
    let serialized = payment.serialize()?;
    assert!(!serialized.is_empty());

    // Test deserialization
    let deserialized = deserialize_payments(&serialized)?;
    assert_eq!(deserialized.len(), 1);
    assert_eq!(deserialized[0].spendable, payment.spendable);
    assert_eq!(deserialized[0].output.value, payment.output.value);
    assert_eq!(deserialized[0].output.script_pubkey, payment.output.script_pubkey);

    Ok(())
}

#[wasm_bindgen_test]
fn test_multiple_payments_serialization() -> Result<()> {
    let dummy_txid1 = Txid::from_raw_hash(sha256d::Hash::hash(&[1; 32]));
    let dummy_txid2 = Txid::from_raw_hash(sha256d::Hash::hash(&[2; 32]));
    
    let payments = vec![
        Payment {
            spendable: OutPoint::new(dummy_txid1, 0),
            output: TxOut {
                value: Amount::from_sat(1000),
                script_pubkey: ScriptBuf::new()
            }
        },
        Payment {
            spendable: OutPoint::new(dummy_txid2, 1),
            output: TxOut {
                value: Amount::from_sat(2000),
                script_pubkey: ScriptBuf::new()
            }
        }
    ];

    // Serialize each payment and combine
    let mut combined = Vec::new();
    for payment in &payments {
        combined.extend(payment.serialize()?);
    }

    // Deserialize and verify
    let deserialized = deserialize_payments(&combined)?;
    assert_eq!(deserialized.len(), 2);
    
    for (original, decoded) in payments.iter().zip(deserialized.iter()) {
        assert_eq!(decoded.spendable, original.spendable);
        assert_eq!(decoded.output.value, original.output.value);
        assert_eq!(decoded.output.script_pubkey, original.output.script_pubkey);
    }

    Ok(())
}

#[wasm_bindgen_test]
fn test_empty_payments_deserialization() -> Result<()> {
    let empty_vec = Vec::new();
    let deserialized = deserialize_payments(&empty_vec)?;
    assert_eq!(deserialized.len(), 0);
    Ok(())
} 