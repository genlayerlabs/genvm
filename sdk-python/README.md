# Genlayer SDK for Python

## Development

To speed up development following feature was implemented: `sdk-debug`. It speeds up wasm compilation by not freezing genlayer sdk into `genvm-python.wasm`. Instead you can run [`build-debug-sdk.sh`](./build-debug-sdk.sh) to automatically produce `./target/sdk.frozen` which then will be loaded by `genvm-python.wasm` built with `sdk-debug` feature from `/sdk.frozen`
