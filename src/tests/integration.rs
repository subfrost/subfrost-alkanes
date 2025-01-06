#[cfg(test)]
mod tests {
    use alkanes::precompiled::{alkanes_std_auth_token_build, alkanes_std_owned_token_build};
    use crate::precompiled::{owned_token_build};
    use crate::tests::std::dx_btc_build;
    use alkanes_support::{cellpack::Cellpack, constants::AUTH_TOKEN_FACTORY_ID};
    use alkanes_support::id::AlkaneId;
    use anyhow::{Result, anyhow};
    use bitcoin::OutPoint;
    use protorune::balance_sheet::load_sheet;
    use protorune::tables::RuneTable;
    use protorune::message::MessageContext;
    use alkanes::message::AlkaneMessageContext;
    use metashrew_support::utils::consensus_encode;
    use metashrew_support::index_pointer::KeyValuePointer;
    use alkanes::tests::helpers::{self as alkane_helpers, assert_binary_deployed_to_id, clear};
    use wasm_bindgen_test::wasm_bindgen_test;
    use alkanes::indexer::index_block;
    #[allow(unused_imports)]
    use metashrew::{
        index_pointer::IndexPointer,
        println,
        stdio::{stdout, Write},
    };
    const ALKANE_FACTORY_OWNED_TOKEN_ID: u128 =  0x0fff;
    const USER_AUTH_TOKEN_AMOUNT: u128 = 100;
    const VAULT_DEPOSIT_AMOUNT: u128 = 50;

    #[wasm_bindgen_test]
    fn test_vault() -> Result<()> {
        clear();
        let block_height = 840_000;

        println!("🏦 Initializing test with block height: {}", block_height);

        // Initialize auth token factory
        let auth_token_factory = Cellpack {
            target: AlkaneId { block: 3, tx: AUTH_TOKEN_FACTORY_ID },
            inputs: vec![100], // Initial supply for factory
        };

        println!("🏭 Created auth token factory with supply: {:?}", auth_token_factory.inputs);

        // Mint auth tokens to user
        let mint_to_user = Cellpack {
            target: AlkaneId { block: 1, tx: 0 },
            inputs: vec![
                0,                  // opcode for minting
                USER_AUTH_TOKEN_AMOUNT,  // Amount to mint to user
            ],
        };

        println!("💫 Minting auth tokens to user: {:?}", mint_to_user.inputs);

        // Create vault and deposit tokens
        let vault_deposit = Cellpack {
            target: AlkaneId { block: 6, tx: ALKANE_FACTORY_OWNED_TOKEN_ID },
            inputs: vec![
                1,                    // opcode for deposit
                VAULT_DEPOSIT_AMOUNT, // Amount to deposit
                0x414243,            // Vault identifier
                0x414243,            // User identifier
            ],
        };

        println!("🔐 Creating vault deposit with amount: {:?}", vault_deposit.inputs);

        let test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
            [
                alkanes_std_auth_token_build::get_bytes(),
                owned_token_build::get_bytes(),
                [].into(),
                dx_btc_build::get_bytes()
            ].into(), 
            vec![auth_token_factory, mint_to_user, vault_deposit]
        );

        println!("🔨 Indexing block...");
        index_block(&test_block, block_height)?;

        // Verify user's remaining balance
        let tx = test_block.txdata.last().ok_or(anyhow!("no last el"))?;
        let user_outpoint = OutPoint {
            txid: tx.compute_txid(),
            vout: 0,
        };

        println!("🔍 Checking user's balance sheet...");
        
        let user_sheet = load_sheet(
            &RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
                .OUTPOINT_TO_RUNES
                .select(&consensus_encode(&user_outpoint)?),
        );

        let remaining_balance = USER_AUTH_TOKEN_AMOUNT - VAULT_DEPOSIT_AMOUNT;
        println!("💰 User's remaining balance: {}", remaining_balance);

        // Verify vault's balance
        let vault_outpoint = OutPoint {
            txid: tx.compute_txid(),
            vout: 1, // Vault's output
        };

        println!("🔍 Checking vault's balance sheet...");
        
        let vault_sheet = load_sheet(
            &RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
                .OUTPOINT_TO_RUNES
                .select(&consensus_encode(&vault_outpoint)?),
        );

        println!("🏦 Vault's balance: {}", VAULT_DEPOSIT_AMOUNT);

        // Verify the balances
        assert_eq!(user_sheet.get(&AlkaneId { block: 2, tx: 1 }.into()), remaining_balance);
        assert_eq!(vault_sheet.get(&AlkaneId { block: 2, tx: 1 }.into()), VAULT_DEPOSIT_AMOUNT);

        println!("✨ Test completed successfully!");

        Ok(())
    }

    #[wasm_bindgen_test]
    fn test_auth_and_owned_token() -> Result<()> {
        clear();
        let block_height = 840_000;

        println!("🏦 Initializing test with block height: {}", block_height);

        let auth_cellpack = Cellpack {
            target: AlkaneId {
                block: 3,
                tx: AUTH_TOKEN_FACTORY_ID,
            },
            inputs: vec![100],
        };

        println!("📝 Created auth token cellpack with input: {:?}", auth_cellpack.inputs);

        let test_cellpack = Cellpack {
            target: AlkaneId { block: 1, tx: 0 },
            inputs: vec![
                0,    /* opcode (to init new auth token) */
                1,    /* auth_token units */
                1000, /* owned_token token_units */
            ],
        };

        println!("📝 Created test cellpack with inputs: {:?}", test_cellpack.inputs);

        let test_block = alkane_helpers::init_with_multiple_cellpacks_with_tx(
            [
                alkanes_std_auth_token_build::get_bytes(),
                alkanes_std_owned_token_build::get_bytes(),
            ]
            .into(),
            [auth_cellpack, test_cellpack].into(),
        );

        println!("🔨 Indexing block...");
        index_block(&test_block, block_height)?;

        let _auth_token_id_factory = AlkaneId {
            block: 4,
            tx: AUTH_TOKEN_FACTORY_ID,
        };

        let auth_token_id_deployment = AlkaneId { block: 2, tx: 2 };
        let owned_token_id = AlkaneId { block: 2, tx: 1 };

        let tx = test_block.txdata.last().ok_or(anyhow!("no last el"))?;
        let outpoint = OutPoint {
            txid: tx.compute_txid(),
            vout: 0,
        };

        println!("🔍 Checking balance sheet for outpoint: {:?}", outpoint);
        
        let sheet = load_sheet(
            &RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
                .OUTPOINT_TO_RUNES
                .select(&consensus_encode(&outpoint)?),
        );

        println!("💰 Owned token balance: {}", sheet.get(&owned_token_id.into()));
        println!("🔑 Auth token balance: {}", sheet.get(&auth_token_id_deployment.into()));

        assert_eq!(sheet.get(&owned_token_id.into()), 1000);
        assert_eq!(sheet.get(&auth_token_id_deployment.into()), 1);

        let tx_first = test_block.txdata.first().ok_or(anyhow!("no first el"))?;
        let outpoint_first = OutPoint {
            txid: tx_first.compute_txid(),
            vout: 0,
        };

        println!("🔍 Checking first transaction outpoint: {:?}", outpoint_first);
        
        let sheet_first = load_sheet(
            &RuneTable::for_protocol(AlkaneMessageContext::protocol_tag())
                .OUTPOINT_TO_RUNES
                .select(&consensus_encode(&outpoint_first)?),
        );

        println!("📊 First transaction balance sheet length: {}", sheet_first.balances.len());

        assert_eq!(sheet_first.balances.len(), 0);

        println!("✅ Verifying binary deployments...");
        
        let _ = assert_binary_deployed_to_id(
            owned_token_id.clone(),
            alkanes_std_owned_token_build::get_bytes(),
        );
        let _ = assert_binary_deployed_to_id(
            _auth_token_id_factory.clone(),
            alkanes_std_auth_token_build::get_bytes(),
        );
        let _ = assert_binary_deployed_to_id(
            auth_token_id_deployment.clone(),
            alkanes_std_auth_token_build::get_bytes(),
        );

        println!("✨ Test completed successfully!");

        Ok(())
    }
}

