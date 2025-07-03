use std::num::NonZero;

use cainome::cairo_serde::U256;
use starkbiter_bindings::{
    contracts_router_lite,
    contracts_swapper::{self, SwapData, I129},
    ekubo_core,
    erc_20_mintable_oz0::Erc20MintableOZ0,
    EKUBO_ROUTER_LITE_CONTRACT_SIERRA, SWAPPER_CONTRACT_SIERRA,
};
use starkbiter_core::{
    environment::Environment,
    middleware::{connection::Connection, traits::Middleware, StarkbiterMiddleware},
    tokens::{get_token_data, TokenId},
};
use starknet::signers::{LocalWallet, SigningKey};
use starknet_accounts::{Account, SingleOwnerAccount};
use starknet_core::{
    types::{Call, Felt},
    utils::get_selector_from_name,
};
use starknet_devnet_core::constants::{self, ARGENT_CONTRACT_CLASS_HASH};
use starknet_devnet_types::{chain_id::ChainId, rpc::gas_modification::GasModificationRequest};
use url::Url;

const ALL_GAS_1: GasModificationRequest = GasModificationRequest {
    gas_price_wei: NonZero::new(1_u128),
    gas_price_fri: NonZero::new(1_u128),

    data_gas_price_wei: NonZero::new(1_u128),
    data_gas_price_fri: NonZero::new(1_u128),

    l2_gas_price_wei: NonZero::new(1_u128),
    l2_gas_price_fri: NonZero::new(1_u128),

    generate_block: Some(true),
};

const MAINNET_EKUBO_CORE_CONTRACT_ADDRESS: &str =
    "0x00000005dd3D2F4429AF886cD1a3b08289DBcEa99A294197E9eB43b0e0325b4b";

pub fn setup_log() {
    std::env::set_var("RUST_LOG", "trace");
    let _ = tracing_subscriber::fmt::try_init();
}

async fn deploy_swapper(
    client: &StarkbiterMiddleware,
    account: &SingleOwnerAccount<Connection, LocalWallet>,
) -> Felt {
    let swapper_class_hash = client
        .declare_contract(SWAPPER_CONTRACT_SIERRA)
        // .declare_contract(EKUBO_ROUTER_LITE_CONTRACT_SIERRA)
        .await
        .unwrap();

    let deploy_call = vec![Call {
        to: constants::UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            swapper_class_hash,                                            // class hash
            Felt::from_hex_unchecked("0x123"),                             // salt
            Felt::ZERO,                                                    // unique
            Felt::TWO,                                                     // constructor length
            Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS), /* core ekubo contraict address */
            account.address(),                                             // withdrawal address
        ],
    }];

    let result = account.execute_v3(deploy_call).send().await.unwrap();

    let swapper_address = client
        .get_deployed_contract_address(result.transaction_hash)
        .await
        .unwrap();

    return swapper_address;
}

async fn deploy_router(
    client: &StarkbiterMiddleware,
    account: &SingleOwnerAccount<Connection, LocalWallet>,
) -> Felt {
    let swapper_class_hash = client
        .declare_contract(EKUBO_ROUTER_LITE_CONTRACT_SIERRA)
        .await
        .unwrap();

    let deploy_call = vec![Call {
        to: constants::UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            swapper_class_hash,                                            // class hash
            Felt::from_hex_unchecked("0x123"),                             // salt
            Felt::ZERO,                                                    // unique
            Felt::ONE,                                                     // constructor length
            Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS), /* core ekubo contraict address */
        ],
    }];

    let result = account.execute_v3(deploy_call).send().await.unwrap();

    let swapper_address = client
        .get_deployed_contract_address(result.transaction_hash)
        .await
        .unwrap();

    return swapper_address;
}

// TODO: optimize tick spacing and trading size for faster trades.
#[tokio::test]
async fn test_ekubo_swap_1eth_for_usdc_with_swapper() {
    // setup_log();

    let fraction_of_eth = 1_00000000_00000000_00000_u128;

    // Custom chain ID for Starknet
    let chain_id = ChainId::Mainnet;

    // Spin up a new Mainnet fork.
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .with_fork(
            Url::parse("https://starknet-mainnet.public.blastapi.io").unwrap(),
            1521205,
            Felt::from_hex_unchecked(
                "0x7aabf76192d3d16fe8bda54c0e7d0a9843c21fe20dd23704366bad38d57dc30",
            ),
        )
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();
    client.set_next_block_gas(ALL_GAS_1).await.unwrap();

    let account = client
        .create_single_owner_account(
            Option::<SigningKey>::None,
            ARGENT_CONTRACT_CLASS_HASH,
            1000000000,
        )
        .await
        .unwrap();

    let core = ekubo_core::EkuboCore::new(
        Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
        &account,
    );

    let eth_token = get_token_data(&chain_id, &TokenId::ETH).unwrap();
    let usdc_token = get_token_data(&chain_id, &TokenId::USDC).unwrap();

    let key = ekubo_core::PoolKey {
        token0: cainome::cairo_serde::ContractAddress::from(eth_token.l2_token_address),
        token1: cainome::cairo_serde::ContractAddress::from(usdc_token.l2_token_address),
        fee: 17014118346046923173168730371588410572,
        tick_spacing: 95310,
        extension: cainome::cairo_serde::ContractAddress::from(Felt::ZERO),
    };

    let pool_price = core.get_pool_price(&key).call().await.unwrap();

    println!("Pool price: {:?}", pool_price);

    let swapper_address = deploy_swapper(&client, &account).await;
    let swapper = contracts_swapper::ContractsSwapper::new(swapper_address, &account);

    let withdrawal_address = swapper.get_withdraw_address().call().await.unwrap();
    println!("Withdrawal address: {:?}", withdrawal_address);

    client
        .top_up_balance(swapper_address, fraction_of_eth, TokenId::ETH)
        .await
        .unwrap();

    if eth_token.l2_token_address > usdc_token.l2_token_address {
        panic!("ETH token address must be less than USDC token address");
    }

    println!(
        "eth_token.l2_token_address = {:?}",
        eth_token.l2_token_address
    );
    println!(
        "usdc_token.l2_token_address = {:?}",
        usdc_token.l2_token_address
    );

    let token0 = eth_token.l2_token_address;

    let swap_one_eth_for_usdc = SwapData {
        pool_key: contracts_swapper::PoolKey {
            token0: key.token0,
            token1: key.token1,
            fee: key.fee,
            tick_spacing: key.tick_spacing,
            extension: key.extension,
        },
        sqrt_ratio_limit: U256 {
            low: 18446748437148339061,
            high: 0,
        }, /* actually sqrt(token1/token0) so sqrt(3000) would be enough, but I don't want to
            * calculate fp value here */
        amount: I129 {
            mag: fraction_of_eth,
            sign: false,
        },
        token: cainome::cairo_serde::ContractAddress::from(token0),
    };

    let usdc_erc20 = Erc20MintableOZ0::new(usdc_token.l2_token_address, &account);
    let eth_erc20 = Erc20MintableOZ0::new(eth_token.l2_token_address, &account);

    async fn get_balance(
        erc20: &Erc20MintableOZ0<&SingleOwnerAccount<Connection, LocalWallet>>,
        address: &Felt,
    ) -> U256 {
        erc20
            .balanceOf(&cainome::cairo_serde::ContractAddress::from(*address))
            .call()
            .await
            .unwrap()
    }

    client.create_block().await.unwrap();

    let initial_usdc_core_balance = get_balance(
        &usdc_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        get_balance(&usdc_erc20, &swapper_address).await == to_u256(0),
        "Swapper USDC balance should be zero before swap"
    );

    let initial_eth_core_balance = get_balance(
        &eth_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        get_balance(&eth_erc20, &swapper_address).await == to_u256(fraction_of_eth),
        "Swaper ETH balance should be 0.1 ETH before swap"
    );

    let res = swapper.swap(&swap_one_eth_for_usdc).send().await;

    if let Err(e) = res {
        assert!(false, "Swap failed {:?}", e);
        return;
    }
    client.create_block().await.unwrap();

    let account_usdc_balance = get_balance(&usdc_erc20, &account.address()).await;

    assert!(
        account_usdc_balance > to_u256(0),
        "Account USDC balance should be greater than zero after swap"
    );

    let core_usdc_balance = get_balance(
        &usdc_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        core_usdc_balance + account_usdc_balance == initial_usdc_core_balance,
        "EKUBO USDC balance should be the difference between pre swap and post swap values"
    );

    let swapper_eth_balance = get_balance(&eth_erc20, &swapper_address).await;

    assert!(
        swapper_eth_balance == to_u256(0),
        "Swapper ETH balance should be zero after swap {:?}",
        swapper_eth_balance
    );

    let ekubo_eth_balance = get_balance(
        &eth_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        ekubo_eth_balance == initial_eth_core_balance + to_u256(fraction_of_eth),
        "EKUBO ETH balance should be one increase by 0.1 eth after swap"
    );

    // To swap I need.
    // Fund swapper contract
    // Exeute swap
    // Check my balance.

    // let swapper = contracts_swapper::ContractsSwapper::new(swapper_address,
    // &account);

    let _ = env.stop();
}

fn to_u256(value: u128) -> U256 {
    U256 {
        low: value,
        high: 0,
    }
}

#[tokio::test]
async fn test_ekubo_swap_1eth_for_usdc_with_router() {
    // setup_log();

    let fraction_of_eth = 1_00000000_00000000_00000_u128;

    // Custom chain ID for Starknet
    let chain_id = ChainId::Mainnet;

    // Spin up a new Mainnet fork.
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .with_fork(
            Url::parse("https://starknet-mainnet.public.blastapi.io").unwrap(),
            1521205,
            Felt::from_hex_unchecked(
                "0x7aabf76192d3d16fe8bda54c0e7d0a9843c21fe20dd23704366bad38d57dc30",
            ),
        )
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();
    client.set_next_block_gas(ALL_GAS_1).await.unwrap();

    let account = client
        .create_single_owner_account(
            Option::<SigningKey>::None,
            ARGENT_CONTRACT_CLASS_HASH,
            1000000000,
        )
        .await
        .unwrap();

    let core = ekubo_core::EkuboCore::new(
        Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
        &account,
    );

    println!(
        "Ekubo Core address: {:?}",
        Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS)
    );

    println!("Account address: {:?}", account.address());

    let eth_token = get_token_data(&chain_id, &TokenId::ETH).unwrap();
    let usdc_token = get_token_data(&chain_id, &TokenId::USDC).unwrap();
    let usdc_erc20 = Erc20MintableOZ0::new(usdc_token.l2_token_address, &account);
    let eth_erc20 = Erc20MintableOZ0::new(eth_token.l2_token_address, &account);

    let key = ekubo_core::PoolKey {
        token0: cainome::cairo_serde::ContractAddress::from(eth_token.l2_token_address),
        token1: cainome::cairo_serde::ContractAddress::from(usdc_token.l2_token_address),
        fee: 17014118346046923173168730371588410572,
        tick_spacing: 95310,
        extension: cainome::cairo_serde::ContractAddress::from(Felt::ZERO),
    };

    let pool_price = core.get_pool_price(&key).call().await.unwrap();
    let pool_liquidity = core.get_pool_liquidity(&key).call().await.unwrap();

    println!("Pool price: {:?}", pool_price);
    println!("Pool liquidity: {:?}", pool_liquidity);

    let router_address = deploy_router(&client, &account).await;
    let router = contracts_router_lite::ContractsRouterLite::new(router_address, &account);

    client
        .top_up_balance(router_address, fraction_of_eth, TokenId::ETH)
        .await
        .unwrap();

    if eth_token.l2_token_address > usdc_token.l2_token_address {
        panic!("ETH token address must be less than USDC token address");
    }

    async fn get_balance(
        erc20: &Erc20MintableOZ0<&SingleOwnerAccount<Connection, LocalWallet>>,
        address: &Felt,
    ) -> U256 {
        erc20
            .balanceOf(&cainome::cairo_serde::ContractAddress::from(*address))
            .call()
            .await
            .unwrap()
    }

    client.create_block().await.unwrap();

    // println!("USDC decimals: {:?}", usdc_token.decimals);
    // println!("ETH decimals: {:?}", eth_token.decimals);

    let initial_usdc_core_balance = get_balance(
        &usdc_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        get_balance(&usdc_erc20, &router_address).await == to_u256(0),
        "Router USDC balance should be zero before swap"
    );

    let initial_eth_core_balance = get_balance(
        &eth_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    // println!("Initial USDC core balance: {:?}", initial_usdc_core_balance);
    // println!("Initial ETH core balance: {:?}", initial_eth_core_balance);

    assert!(
        get_balance(&eth_erc20, &router_address).await == to_u256(fraction_of_eth),
        "Router ETH balance should be 0.1 ETH before swap"
    );

    let res = router
        .swap(
            &contracts_router_lite::RouteNode {
                pool_key: contracts_router_lite::PoolKey {
                    token0: key.token0,
                    token1: key.token1,
                    fee: key.fee,
                    tick_spacing: key.tick_spacing,
                    extension: key.extension,
                },
                sqrt_ratio_limit: U256 {
                    low: 18446748437148339061,
                    high: 0,
                },
                skip_ahead: 0,
            },
            &contracts_router_lite::TokenAmount {
                token: cainome::cairo_serde::ContractAddress::from(key.token0),
                amount: contracts_router_lite::I129 {
                    mag: fraction_of_eth,
                    sign: false,
                },
            },
        )
        .send()
        .await;

    if let Err(e) = res {
        assert!(false, "Swap failed with error: {:?}", e);
        return;
    }
    client.create_block().await.unwrap();

    let router_usdc_balance = get_balance(&usdc_erc20, &router_address).await;

    assert!(
        router_usdc_balance > to_u256(0),
        "Router USDC balance should be greater than zero after swap"
    );

    let core_usdc_balance = get_balance(
        &usdc_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        core_usdc_balance + router_usdc_balance == initial_usdc_core_balance,
        "EKUBO USDC balance should be the difference between pre swap and post swap values"
    );

    let router_eth_balance = get_balance(&eth_erc20, &router_address).await;

    assert!(
        router_eth_balance == to_u256(0),
        "Router ETH balance should be zero after swap {:?}",
        router_eth_balance
    );

    let ekubo_eth_balance = get_balance(
        &eth_erc20,
        &Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS),
    )
    .await;

    assert!(
        ekubo_eth_balance == initial_eth_core_balance + to_u256(fraction_of_eth),
        "EKUBO ETH balance should be one increase by 0.1 eth after swap"
    );

    let _ = env.stop();
}
