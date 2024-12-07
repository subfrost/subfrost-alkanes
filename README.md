# frBTC


## Build

```sh
cargo build --release
```

WASM will be built to `target/wasm32-unknown-unknown/fr_btc.wasm`

gzip compression level 9 is recommended to compress the wasm to a `*.wasm.gz` file before deploying to Bitcoin.

## Usage

This alkane implements the following opcodes:

- 0: `initialize(mint_auth_token_amount: u128, mint_amount: u128, rune_id_for_runes_stable: u128[2])`
- 77: `mint()`
- 78: `burn(u128)`
- 99: `name(): String`
- 100: `symbol(): String`
- 1001: `payments_at_height(): Vec<u8>`


## Author

flex

## License

MIT
