//! Synthetic Bitcoin (frBTC) Contract
//!
//! A synthetic representation of Bitcoin on the Subfrost protocol.
//! It allows users to wrap their BTC into frBTC and unwrap frBTC back to BTC.
//! The contract verifies Bitcoin transactions to ensure proper wrapping and unwrapping.

use alkanes_runtime::auth::AuthenticatedResponder;
use alkanes_runtime::{
    declare_alkane, message::MessageDispatch, runtime::AlkaneResponder, storage::StoragePointer,
};
#[allow(unused_imports)]
use alkanes_runtime::{
    println,
    stdio::{stdout, Write},
};
use alkanes_std_factory_support::MintableToken;
use alkanes_support::id::AlkaneId;
use alkanes_support::{context::Context, parcel::AlkaneTransfer, response::CallResponse};
use anyhow::{anyhow, Result};
use bitcoin::hashes::Hash;
use bitcoin::key::TapTweak;
use bitcoin::secp256k1::{self, XOnlyPublicKey};
use bitcoin::{Amount, OutPoint, ScriptBuf, Transaction, TxOut};
use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::{compat::to_arraybuffer_layout, utils::consensus_decode};
use ordinals::{Artifact, Runestone};
use protorune_support::{
    network::{set_network, NetworkParams},
    protostone::Protostone,
};
use std::sync::Arc;
use types_support::Payment;

/// Default signer pubkey for testnet
#[cfg(feature = "testnet")]
pub const DEFAULT_SIGNER_PUBKEY: [u8; 32] = [
    0x07, 0x9a, 0x54, 0xd0, 0xae, 0xf2, 0xb3, 0x43, 0xaa, 0xc8, 0x9c, 0x0f, 0xd7, 0x89, 0xaa, 0xb4,
    0xac, 0xb9, 0x1f, 0x00, 0xca, 0xa0, 0xf8, 0xd5, 0x15, 0x01, 0x45, 0x2c, 0xe4, 0x7c, 0xc9, 0x7d,
];

/// Default signer pubkey for all other networks (zeros)
#[cfg(not(feature = "testnet"))]
pub const DEFAULT_SIGNER_PUBKEY: [u8; 32] = [
    0x07, 0x9a, 0x54, 0xd0, 0xae, 0xf2, 0xb3, 0x43, 0xaa, 0xc8, 0x9c, 0x0f, 0xd7, 0x89, 0xaa, 0xb4,
    0xac, 0xb9, 0x1f, 0x00, 0xca, 0xa0, 0xf8, 0xd5, 0x15, 0x01, 0x45, 0x2c, 0xe4, 0x7c, 0xc9, 0x7d,
];

/// Extension trait for Context to add transaction_id method

#[derive(Default)]
pub struct SyntheticBitcoin(());

/// Message enum for opcode-based dispatch
#[derive(MessageDispatch)]
enum SyntheticBitcoinMessage {
    /// Initialize the contract with auth tokens
    #[opcode(0)]
    Initialize,

    /// Set the signer script pubkey
    #[opcode(1)]
    SetSigner {
        /// Output index in the transaction
        vout: u128,
    },

    /// Wrap BTC to frBTC
    #[opcode(77)]
    Wrap,

    /// Unwrap frBTC to BTC
    #[opcode(78)]
    Unwrap {
        /// Output index in the transaction
        vout: u128,
    },

    /// Set the premium value (owner only)
    #[opcode(4)]
    SetPremium {
        /// Premium value (0-100000000)
        premium: u128,
    },

    /// Get the signer address
    #[opcode(103)]
    #[returns(Vec<u8>)]
    GetSigner,

    /// Get pending payments
    #[opcode(101)]
    #[returns(Vec<u8>)]
    GetPendingPayments,

    /// Get token name
    #[opcode(99)]
    #[returns(String)]
    GetName,

    /// Get token symbol
    #[opcode(100)]
    #[returns(String)]
    GetSymbol,

    #[opcode(101)]
    #[returns(u128)]
    GetTotalSupply,

    /// Get token decimals
    #[opcode(102)]
    #[returns(u8)]
    GetDecimals,

    /// Get the current premium value
    #[opcode(104)]
    #[returns(u128)]
    GetPremium,
}

/// Configure the network parameters for the Bitcoin network.
/// This function sets the appropriate network parameters based on the build features.
/// By default, it uses regtest parameters.
#[cfg(all(
    not(feature = "mainnet"),
    not(feature = "testnet"),
    not(feature = "luckycoin"),
    not(feature = "dogecoin"),
    not(feature = "bellscoin")
))]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("bcrt"),
        p2pkh_prefix: 0x64,
        p2sh_prefix: 0xc4,
    });
}
#[cfg(feature = "mainnet")]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("bc"),
        p2sh_prefix: 0x05,
        p2pkh_prefix: 0x00,
    });
}
#[cfg(feature = "testnet")]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("tb"),
        p2pkh_prefix: 0x6f,
        p2sh_prefix: 0xc4,
    });
}
#[cfg(feature = "luckycoin")]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("lky"),
        p2pkh_hash: 0x2f,
        p2sh_hash: 0x05,
    });
}

#[cfg(feature = "dogecoin")]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("dc"),
        p2pkh_hash: 0x1e,
        p2sh_hash: 0x16,
    });
}
#[cfg(feature = "bellscoin")]
pub fn configure_network() {
    set_network(NetworkParams {
        bech32_prefix: String::from("bel"),
        p2pkh_hash: 0x19,
        p2sh_hash: 0x1e,
    });
}

#[cfg(all(
    not(feature = "mainnet"),
    not(feature = "testnet"),
    not(feature = "luckycoin"),
    not(feature = "dogecoin"),
    not(feature = "bellscoin")
))]
pub fn get_auth_token() -> AlkaneId {
    AlkaneId { block: 4, tx: 123 }
}
#[cfg(feature = "mainnet")]
pub fn get_auth_token() -> AlkaneId {
    AlkaneId { block: 32, tx: 1 }
}

/// Add decimals as a regular method, not part of the Token trait
impl SyntheticBitcoin {
    fn decimals(&self) -> u8 {
        8u8 // Same as Bitcoin
    }
}
impl MintableToken for SyntheticBitcoin {}

// First implement AlkaneResponder for SyntheticBitcoin
impl AlkaneResponder for SyntheticBitcoin {}

impl AuthenticatedResponder for SyntheticBitcoin {
    fn auth_token(&self) -> Result<AlkaneId> {
        Ok(AlkaneId { block: 32, tx: 1 })
    }
}

// Use the MessageDispatch macro for opcode handling
declare_alkane! {
    impl AlkaneResponder for SyntheticBitcoin {
        type Message = SyntheticBitcoinMessage;
    }
}

impl SyntheticBitcoin {
    /// Get the storage pointer for the premium value
    fn premium_pointer(&self) -> StoragePointer {
        StoragePointer::from_keyword("/premium")
    }

    /// Get the current premium value (0-100000000)
    fn premium(&self) -> u128 {
        let value = self.premium_pointer().get();
        if value.len() == 0 {
            500_000 // Default to 0.5% (1e8 / 200)
        } else {
            let mut bytes = [0u8; 16];
            let len = std::cmp::min(value.len(), 16);
            bytes[0..len].copy_from_slice(&value.as_ref()[0..len]);
            u128::from_le_bytes(bytes)
        }
    }

    /// Set the premium value internally (0-100000000)
    fn set_premium_internal(&self, premium: u128) -> Result<()> {
        // Ensure premium is within valid range (0 to 1 BTC in satoshis)
        if premium > 100_000_000 {
            return Err(anyhow!("Premium must be between 0 and 100,000,000"));
        }

        self.premium_pointer()
            .set(Arc::new(premium.to_le_bytes().to_vec()));
        Ok(())
    }

    /// Get the storage pointer for the signer's script pubkey
    fn signer_pointer(&self) -> StoragePointer {
        StoragePointer::from_keyword("/signer")
    }

    /// Get the signer's script pubkey
    /// Returns the stored signer if set, otherwise returns the default signer pubkey
    fn signer(&self) -> Vec<u8> {
        let stored_signer = self.signer_pointer().get();
        if stored_signer.len() > 0 {
            stored_signer.as_ref().clone()
        } else {
            DEFAULT_SIGNER_PUBKEY.to_vec()
        }
    }

    /// Set the signer's script pubkey from a transaction output (internal implementation)
    /// # Arguments
    /// * `context` - The context of the call
    /// * `_vout` - The output index in the transaction
    ///
    /// # Returns
    /// Result indicating success or failure
    fn set_signer_internal(&self, context: &Context, _vout: u128) -> Result<()> {
        let vout = _vout as usize;
        let tx = consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?;

        if let Some(Artifact::Runestone(ref runestone)) = Runestone::decipher(&tx) {
            let protostones = Protostone::from_runestone(runestone)?;
            let message = &protostones[(context.vout as usize) - tx.output.len() - 1];

            if message.edicts.len() != 0 {
                return Err(anyhow!("message cannot contain edicts, only a pointer"));
            }

            let pointer = message
                .pointer
                .ok_or("")
                .map_err(|_| anyhow!("no pointer in message"))?;

            if pointer as usize >= tx.output.len() {
                return Err(anyhow!("pointer cannot be a protomessage"));
            }

            if pointer as usize == vout {
                return Err(anyhow!(
                    "pointer cannot be equal to output spendable by synthetic"
                ));
            }

            self.signer_pointer().set(Arc::new(
                tx.output[vout as usize].script_pubkey.as_bytes().to_vec(),
            ));
            Ok(())
        } else {
            Err(anyhow!(
                "unexpected condition: execution occurred with no Protostone present"
            ))
        }
    }

    /// Check if a transaction has already been processed
    ///
    /// # Arguments
    /// * `tx` - The transaction to check
    ///
    /// # Returns
    /// Result indicating if the transaction is new (Ok) or already processed (Err)
    fn observe_transaction(&self, tx: &Transaction) -> Result<()> {
        let txid = tx.compute_txid();

        let mut ptr = StoragePointer::from_keyword("/seen/").select(&txid.as_byte_array().to_vec());
        if ptr.get().len() != 0 {
            Err(anyhow!("transaction already processed"))
        } else {
            ptr.set_value::<u8>(0x01);
            Ok(())
        }
    }

    /// Compute the total output value sent to the signer
    ///
    /// # Arguments
    /// * `tx` - The transaction to compute outputs for
    ///
    /// # Returns
    /// The total value sent to the signer
    fn compute_output(&self, tx: &Transaction) -> u128 {
        let signer_pubkey_bytes = self.signer();
        let signer_pubkey =
            XOnlyPublicKey::from_slice(&signer_pubkey_bytes).expect("Invalid x-only pubkey");
        let secp = secp256k1::Secp256k1::new();
        let (tweaked_pubkey, _) = signer_pubkey.tap_tweak(&secp, None);
        let signer_script = ScriptBuf::new_p2tr_tweaked(tweaked_pubkey);
        let total = tx.output.iter().fold(0, |r: u128, v: &TxOut| -> u128 {
            if v.script_pubkey == signer_script {
                r + <u64 as Into<u128>>::into(v.value.to_sat())
            } else {
                r
            }
        });

        total
    }

    /// Get the amount of frBTC to burn from the incoming alkanes
    ///
    /// # Arguments
    /// * `context` - The context of the call
    ///
    /// # Returns
    /// The amount of frBTC to burn
    fn burn_input(&self, context: &Context) -> Result<u64> {
        let value = context
            .incoming_alkanes
            .0
            .iter()
            .find(|v| context.myself == v.id)
            .ok_or("")
            .map_err(|_| anyhow!("must spend synthetics into message"))?
            .value
            .try_into()?;

        Ok(value)
    }

    /// Burn frBTC and create a payment for unwrapping to BTC
    ///
    /// # Arguments
    /// * `context` - The context of the call
    /// * `vout` - The output index in the transaction
    ///
    /// # Returns
    /// The amount of frBTC burned
    fn burn(&self, context: &Context, vout: usize) -> Result<u64> {
        let tx = consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?;
        let txid = tx.compute_txid();

        if let Some(Artifact::Runestone(ref runestone)) = Runestone::decipher(&tx) {
            let protostones = Protostone::from_runestone(runestone)?;
            let message = &protostones[(context.vout as usize) - tx.output.len() - 1];

            if message.edicts.len() != 0 {
                return Err(anyhow!("message cannot contain edicts, only a pointer"));
            }

            let pointer = message
                .pointer
                .ok_or("")
                .map_err(|_| anyhow!("no pointer in message"))?;

            if pointer as usize >= tx.output.len() {
                return Err(anyhow!("pointer cannot be a protomessage"));
            }

            if pointer as usize == vout {
                return Err(anyhow!(
                    "pointer cannot be equal to output spendable by synthetic"
                ));
            }

            let signer = self.signer();
            if signer != tx.output[vout].script_pubkey.as_bytes().to_vec() {
                return Err(anyhow!(
                    "signer pubkey must be targeted with supplementary output"
                ));
            }

            let value = self.burn_input(context)?;

            // Create a payment record for the unwrap
            let payment = Payment {
                output: TxOut {
                    script_pubkey: tx.output[pointer as usize].script_pubkey.clone(),
                    value: Amount::from_sat(value),
                },
                spendable: OutPoint {
                    txid,
                    vout: vout.try_into()?,
                },
            };
            // Store the payment record
            StoragePointer::from_keyword("/payments/byheight/")
                .select_value(0u64) // Use a fixed height for now
                .append(Arc::<Vec<u8>>::new(payment.serialize()?));
            Ok(value)
        } else {
            Err(anyhow!("execution triggered unexpectedly -- no protostone"))
        }
    }

    /// Wrap BTC to frBTC by verifying a Bitcoin transaction
    ///
    /// # Arguments
    /// * `context` - The context of the call
    ///
    /// # Returns
    /// An AlkaneTransfer representing the minted frBTC
    fn exchange(&self, context: &Context) -> Result<AlkaneTransfer> {
        let tx = consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?;

        // Check if the transaction has already been processed
        self.observe_transaction(&tx)?;

        // Compute the amount of BTC sent to the signer
        let payout = self.compute_output(&tx);

        // Apply premium (subtract fee)
        let premium = self.premium();
        let adjusted_payout = if premium > 0 && payout > 0 {
            // Calculate fee based on premium
            // For each Bitcoin (100,000,000 satoshis), subtract premium amount
            let fee = (payout * premium) / 100_000_000;
            payout.saturating_sub(fee)
        } else {
            payout
        };

        // Mint frBTC tokens with adjusted payout
        let transfer = self.mint(&context, adjusted_payout)?;

        println!("transfer {:?}", transfer);
        Ok(transfer)
    }

    /// Get all pending payments at the current height (internal implementation)
    /// # Returns
    /// A vector of serialized Payment objects
    fn get_pending_payments_internal(&self) -> Vec<u8> {
        let payments = StoragePointer::from_keyword("/payments/byheight/")
            .select_value(0u64) // Use a fixed height for now
            .get_list()
            .into_iter()
            .fold(Vec::<u8>::new(), |r, v| {
                let mut result = Vec::<u8>::with_capacity(r.len() + v.len());
                result.extend(&r);
                result.extend(v.as_ref());
                result
            });
        payments
    }

    /// Initialize the contract with auth tokens
    fn initialize(&self) -> Result<CallResponse> {
        configure_network();
        self.observe_initialization()?;
        let context = self.context()?;
        let response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
        self.set_auth_token(get_auth_token())?;
        self.set_name_and_symbol_str("SUBFROST BTC".to_string(), "frBTC".to_string());
        Ok(response)
    }
    /// Set the signer script pubkey
    fn set_signer(&self, vout: u128) -> Result<CallResponse> {
        configure_network();
        self.only_owner()?;
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        self.set_signer_internal(&context, vout)?;
        response.data = self.signer();
        Ok(response)
    }

    /// Wrap BTC to frBTC
    fn wrap(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        response.alkanes.0.push(self.exchange(&context)?);
        Ok(response)
    }

    /// Unwrap frBTC to BTC
    fn unwrap(&self, vout: u128) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;

        if context.caller.clone() != (AlkaneId { tx: 0, block: 0 }) {
            return Err(anyhow!("must be called by EOA"));
        }

        if context.incoming_alkanes.0.len() != 1
            || context.incoming_alkanes.0[0].id != context.myself
        {
            return Err(anyhow!("must only send frBTC as input"));
        }

        let burn_value = self.burn(&context, vout as usize)?;

        let mut burn_response = CallResponse::default();
        burn_response.data = burn_value.to_le_bytes().to_vec();
        Ok(burn_response)
    }

    /// Get the signer address
    fn get_signer(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        // Always have a signer (either custom or default)
        response.data = self.signer();
        Ok(response)
    }

    /// Get pending payments
    fn get_pending_payments(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut payments = CallResponse::forward(&context.incoming_alkanes);
        payments.data = self.get_pending_payments_internal();
        Ok(payments)
    }

    /// Get token name
    fn get_name(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
        response.data = self.name().into_bytes().to_vec();
        Ok(response)
    }

    /// Get token symbol
    fn get_symbol(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
        response.data = self.symbol().into_bytes().to_vec();
        Ok(response)
    }

    fn get_total_supply(&self) -> Result<CallResponse> {
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes.clone());

        response.data = self.total_supply().to_le_bytes().to_vec();

        Ok(response)
    }

    /// Get token decimals
    fn get_decimals(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);
        response.data = vec![self.decimals()]; // Using the regular method
        Ok(response)
    }

    /// Set the premium value (owner only)
    fn set_premium(&self, premium: u128) -> Result<CallResponse> {
        configure_network();
        self.only_owner()?;
        let context = self.context()?;
        let response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        // Validate and set premium
        if premium > 100_000_000 {
            return Err(anyhow!("Premium must be between 0 and 100,000,000"));
        }

        // Set the premium value
        self.set_premium_internal(premium)?;

        Ok(response)
    }

    /// Get the current premium value
    fn get_premium(&self) -> Result<CallResponse> {
        configure_network();
        let context = self.context()?;
        let mut response: CallResponse = CallResponse::forward(&context.incoming_alkanes);

        // Get the premium value and return it
        let premium = self.premium();
        response.data = premium.to_le_bytes().to_vec();

        Ok(response)
    }
}

// The __execute function is now handled by the declare_alkane! macro

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::secp256k1::{self, XOnlyPublicKey};
    use bitcoin::Script;

    #[test]
    fn test_default_signer_pubkey() {
        let contract = SyntheticBitcoin::default();
        let signer_pubkey_bytes = contract.signer();

        assert_eq!(
            signer_pubkey_bytes,
            DEFAULT_SIGNER_PUBKEY.to_vec(),
            "The default signer should be the default pubkey"
        );
    }
}
