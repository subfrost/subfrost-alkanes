#[cfg(test)]
mod tests {
    use alkanes::precompiled::{alkanes_std_auth_token_build};
    use crate::precompiled::{owned_token_build};
    use crate::tests::std::dx_btc_build;
    use alkanes_support::{cellpack::Cellpack, constants::AUTH_TOKEN_FACTORY_ID};
    use alkanes_support::id::AlkaneId;
    use anyhow::Result;
    use hex;
    use metashrew_support::index_pointer::KeyValuePointer;

    use alkanes::index_block;
    use alkanes::tests::helpers as alkane_helpers;
    use alkane_helpers::clear;
    use alkanes_support::gz::{compress, decompress};
    #[allow(unused_imports)]
    use metashrew::{
	index_pointer::IndexPointer,
	println,
	stdio::{stdout, Write},
    };
    const ALKANE_FACTORY_OWNED_TOKEN_ID: u128 =  0x0fff;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn test_vault() -> Result<()> {
	clear();
	let block_height = 840_000;

	let test_cellpacks = [
	    //create alkane
	    Cellpack {
		target: AlkaneId { block: 3, tx: AUTH_TOKEN_FACTORY_ID },
		inputs: vec![100],
	    },
	    Cellpack {
		target: AlkaneId { block: 3, tx: ALKANE_FACTORY_OWNED_TOKEN_ID },
		inputs: vec![100],
	    },
	    Cellpack {
		target: AlkaneId { block: 6, tx: ALKANE_FACTORY_OWNED_TOKEN_ID },
		inputs: vec![0, 1, 1000000000u128, 0x414243, 0x414243],
	    },
            Cellpack {
                target: AlkaneId { block: 1, tx: 0 },
                inputs: vec![0, 2, 0]
            }
	];

	let test_block = alkane_helpers::init_with_multiple_cellpacks_and_tx([
          alkanes_std_auth_token_build::get_bytes(),
          owned_token_build::get_bytes(),
          [].into(),
          dx_btc_token_build::get_bytes()
        ], test_cellpacks.to_vec());
	index_block(&test_block, block_height as u32)?;
	Ok(())
    }
}

