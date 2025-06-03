use std::sync::Arc;

use arbiter_core::{environment::Environment, middleware::ArbiterMiddleware};
use starknet::{
    macros::selector,
    providers::{Provider, ProviderError},
    signers::{LocalWallet, Signer, SigningKey},
};
use starknet_accounts::{
    Account, AccountError, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount,
};
use starknet_core::{
    chain_id::MAINNET,
    types::{BlockId, BlockTag, Felt, FunctionCall},
    utils::get_storage_var_address,
};
use starknet_devnet_core::constants;
use starknet_devnet_types::{
    felt::join_felts, num_bigint::BigUint, starknet_api::state::StorageKey,
};

pub fn log() {
    std::env::set_var("RUST_LOG", "trace");
    tracing_subscriber::fmt::init();
}

pub fn startup() -> (Environment, Arc<ArbiterMiddleware>) {
    let env = Environment::builder().build();
    let client = ArbiterMiddleware::new(&env).unwrap();
    log();
    (env, client)
}

async fn get_total_supply(client: &Arc<ArbiterMiddleware>) -> BigUint {
    let total_supply = client
        .call(
            FunctionCall {
                contract_address: constants::STRK_ERC20_CONTRACT_ADDRESS, // STRK MAINNET ERC20
                entry_point_selector: selector!("totalSupply"),
                calldata: vec![],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect("failed to call contract");

    return join_felts(&total_supply.get(1).unwrap(), &total_supply.get(0).unwrap());
}

async fn get_total_supply_through_storage(client: &Arc<ArbiterMiddleware>) -> BigUint {
    let total_supply_key: StorageKey = get_storage_var_address("ERC20_total_supply", &[])
        .unwrap()
        .try_into()
        .unwrap();

    let low_val = get_storage_at(
        &client,
        constants::STRK_ERC20_CONTRACT_ADDRESS,
        total_supply_key.into(),
    )
    .await;

    let high_val = get_storage_at(
        &client,
        constants::STRK_ERC20_CONTRACT_ADDRESS,
        total_supply_key.next_storage_key().unwrap().into(),
    )
    .await;

    return join_felts(&high_val, &low_val);
}

async fn get_storage_at(
    client: &Arc<ArbiterMiddleware>,
    contract_address: Felt,
    key: Felt,
) -> Felt {
    let res = client
        .get_storage_at(contract_address, key, BlockId::Tag(BlockTag::Latest))
        .await
        .expect("failed to call contract");

    return res;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_unnamed() {
    log();
    let env = Environment::builder()
        .with_fork(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_8"
                .parse()
                .unwrap(),
            1454859,
            Felt::from_hex("0x01166a9c43a2db7a3c0c2db4089948cdc9b250f1644cf035d53b3defb3b90179")
                .unwrap(), // Block 1
        )
        .build();

    tracing::info!("Environment built");
    let client = ArbiterMiddleware::new(&env).unwrap();

    let signing_key = SigningKey::from_random();
    let public_key = signing_key.verifying_key();

    tracing::info!("TOTAL_SUPPLY: {:?}", get_total_supply(&client).await);
    tracing::info!(
        "TOTAL_SUPPLY2: {:?}",
        get_total_supply_through_storage(&client).await
    );

    let address = client
        .create_account(public_key, constants::ARGENT_CONTRACT_CLASS_HASH)
        .await
        .unwrap();

    tracing::info!(
        "TOTAL_SUPPLY state directly: {:?}",
        get_total_supply(&client).await
    );
    tracing::info!(
        "TOTAL_SUPPLY: {:?}",
        get_total_supply_through_storage(&client).await
    );

    client.create_block().await.unwrap();
    tracing::info!("TOTAL_SUPPLY: {:?}", get_total_supply(&client).await);

    // let middleware_account = SingleOwnerAccount::new(
    //     client.as_ref(),
    //     LocalWallet::from(signing_key),
    //     address.into(),
    //     MAINNET,
    //     ExecutionEncoding::New,
    // );
    // {
    //     let address_to_check_balance =
    //         Felt::from_hex("0x04164013f90b05d67f026779bf96e9c401c96f3485b645a786166e6935fba116")
    //             .unwrap();

    //     tracing::info!("Calling");
    //     let call_result = client
    //         .call(
    //             FunctionCall {
    //                 contract_address: constants::STRK_ERC20_CONTRACT_ADDRESS, // STRK MAINNET ERC20
    //                 entry_point_selector: selector!("balanceOf"),
    //                 calldata: vec![address_to_check_balance],
    //             },
    //             BlockId::Tag(BlockTag::Latest),
    //         )
    //         .await
    //         .expect("failed to call contract");

    //     tracing::info!(
    //         "Call result: {:?} - {:?}",
    //         address_to_check_balance.to_hex_string(),
    //         call_result
    //     );
    // // }

    // {
    //     let address_to_check_balance_for = Felt::from(address);
    //     let call_result = client
    //         .call(
    //             FunctionCall {
    //                 contract_address: constants::STRK_ERC20_CONTRACT_ADDRESS, // STRK MAINNET ERC20
    //                 entry_point_selector: selector!("balanceOf"),
    //                 calldata: vec![address_to_check_balance_for],
    //             },
    //             BlockId::Tag(BlockTag::Latest),
    //         )
    //         .await
    //         .expect("failed to call contract");

    //     let total_supply = client
    //         .call(
    //             FunctionCall {
    //                 contract_address: constants::STRK_ERC20_CONTRACT_ADDRESS, // STRK MAINNET ERC20
    //                 entry_point_selector: selector!("totalSupply"),
    //                 calldata: vec![],
    //             },
    //             BlockId::Tag(BlockTag::Latest),
    //         )
    //         .await
    //         .expect("failed to call contract");

    //     tracing::info!("Call result: Total supply - {:?}", total_supply);
    // }

    // Ok(())
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
