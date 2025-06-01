use std::sync::Arc;

use starknet_accounts::{
    Account, AccountError, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
};

use arbiter_bindings::counter::Counter;
use arbiter_core::{environment::Environment, middleware::ArbiterMiddleware};
use starknet_core::chain_id::MAINNET;

pub fn log() {
    std::env::set_var("RUST_LOG", "trace");
    tracing_subscriber::fmt::init();
}

pub fn startup() -> (Environment, Arc<ArbiterMiddleware>) {
    let env = Environment::builder().build();
    let client = ArbiterMiddleware::new(&env).unwrap();
    (env, client)
}

pub async fn deploy_counter(client: Arc<ArbiterMiddleware>) {
    let account = SingleOwnerAccount::new(client, client, address, MAINNET, ExecutionEncoding::New);

    //     ArbiterToken::deploy(
    //         client,
    //         (
    //             ARBITER_TOKEN_X_NAME.to_string(),
    //             ARBITER_TOKEN_X_SYMBOL.to_string(),
    //             ARBITER_TOKEN_X_DECIMALS,
    //         ),
    //     )
    //     .unwrap()
    //     .send()
    //     .await
    //     .unwrap()
}

// pub async fn deploy_arby(client: Arc<ArbiterMiddleware>) -> ArbiterToken<ArbiterMiddleware> {
//     ArbiterToken::deploy(
//         client,
//         (
//             ARBITER_TOKEN_Y_NAME.to_string(),
//             ARBITER_TOKEN_Y_SYMBOL.to_string(),
//             ARBITER_TOKEN_Y_DECIMALS,
//         ),
//     )
//     .unwrap()
//     .send()
//     .await
//     .unwrap()
// }

// pub async fn deploy_liquid_exchange(
//     client: Arc<ArbiterMiddleware>,
// ) -> (
//     ArbiterToken<ArbiterMiddleware>,
//     ArbiterToken<ArbiterMiddleware>,
//     LiquidExchange<ArbiterMiddleware>,
// ) {
//     let arbx = deploy_arbx(client.clone()).await;
//     let arby = deploy_arby(client.clone()).await;
//     let price = parse_ether(LIQUID_EXCHANGE_PRICE).unwrap();
//     let liquid_exchange = LiquidExchange::deploy(client, (arbx.address(), arby.address(), price))
//         .unwrap()
//         .send()
//         .await
//         .unwrap();
//     (arbx, arby, liquid_exchange)
// }

// pub async fn deploy_arbiter_math(client: Arc<ArbiterMiddleware>) -> ArbiterMath<ArbiterMiddleware> {
//     ArbiterMath::deploy(client, ())
//         .unwrap()
//         .send()
//         .await
//         .unwrap()
// }
