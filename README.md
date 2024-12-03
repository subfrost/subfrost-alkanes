# alkanes-std-telestable

Implementation of a basic mintable asset on alkanes which can be minted either by an owner token or via protoburn of some asset on the runes metaprotocol.

This could conceivably be used to create a stablecoin similar in design to USDC as it exists on Ethereum, but for alkanes, where you also have a ruens representation of the asset which you would honor.

Providing the ability to protoburn the runes representation of the stable on the runes metaprotocol is a mechanism by which you could trustlessly mediate transfer of the stablecoin value without relaying a transfer between the runes metaprotocol and alkanes with a centralized process.

## Build

```sh
cargo build --release
```

WASM will be built to `target/wasm32-unknown-unknown/alkanes-std-telestable.wasm`

gzip compression level 9 is recommended to compress the wasm to a `*.wasm.gz` file before deploying to Bitcoin.

## Usage

This alkane implements the following opcodes:

- 0: `initialize(mint_auth_token_amount: u128, mint_amount: u128, rune_id_for_runes_stable: u128[2])`
- 47: `mint_from_runes()`
- 77: `mint(mint_amount: u128)`
- 99: `name(): String`
- 100: `symbol(): String`


## Author

flex

## License

MIT
