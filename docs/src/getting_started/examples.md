# Examples

We have a few examples to help you get started with Arbiter. These examples are designed to be simple and easy to understand. They are also designed to be easy to run and modify. We hope you find them helpful!

Our example is in the [examples](https://github.com/astraly-labs/starkbiter/tree/main/examples) directory.

## Simulation

You can run it with the following command:

```bash
cargo run --example minter simulate ./examples/minter/config.toml -vvvv
```

This will run the minter simulation. This simulation is rather complex one. There are two agents:
 - Token Admin (*TA*)
 - Token Requested (*TR*)

*TA* creates ERC20 contracts and subscribes to a highlivel messenger that *TR* uses to communicate the intent for minting more tokens. Upon receiving the intent *TA* mints tokens for *TR*. *TR*, in turn, upon creation subscribes to TokenMinted event defined in ERC20 contract implementation and upon noting it being emitted by the devnet requests more token. This effectively creates and endless loop of token minting that is being broken when certain amount of tokens were minted.

## Forking

Forking from command line is not yet supported.
Forking using environment configuration needs to be tested.

But you can fork from code like that:

```rust ignore
let env = Environment::builder()
    .with_chain_id(chain_id.into())
    .with_fork(
        Url::from_str("http://json-rpc-provider-to-fork-from:1234").unwrap(),
        1000, // Block number to fork from
        Some(Felt::from_str("0xblock_hash").unwrap()),
    )
    .build();
```
