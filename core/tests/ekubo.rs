use cainome::cairo_serde::U256;
use starkbiter_bindings::{
    contracts_swapper::{self, SwapData, I129},
    ekubo_core,
    erc_20_mintable_oz0::Erc20MintableOZ0,
    EKUBO_CORE_CONTRACT_SIERRA, SWAPPER_CONTRACT_SIERRA,
};
use starkbiter_core::{
    environment::Environment,
    middleware::{connection::Connection, traits::Middleware, StarkbiterMiddleware},
    tokens::{get_token_data, TokenId},
};
use starknet::signers::{LocalWallet, SigningKey};
use starknet_accounts::{Account, SingleOwnerAccount};
use starknet_devnet_core::constants::{self, ARGENT_CONTRACT_CLASS_HASH};
use std::num::NonZero;
use url::Url;

use starknet_core::{
    types::{Call, Felt},
    utils::get_selector_from_name,
};

use starknet_devnet_types::{chain_id::ChainId, rpc::gas_modification::GasModificationRequest};

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
    tracing_subscriber::fmt::try_init();
}

async fn deploy_swapper(
    client: &StarkbiterMiddleware,
    account: &SingleOwnerAccount<Connection, LocalWallet>,
) -> Felt {
    let swapper_class_hash = client
        .declare_contract(SWAPPER_CONTRACT_SIERRA)
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
            Felt::from_hex_unchecked(MAINNET_EKUBO_CORE_CONTRACT_ADDRESS), // core ekubo contraict address
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

#[tokio::test]
async fn test_create_account_and_use_it_to_deploy_udc_counter_contract() {
    setup_log();

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
    client.create_block().await.unwrap();

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
        fee: 170141183460469235273462165868118016,
        tick_spacing: 1000,
        extension: cainome::cairo_serde::ContractAddress::from(Felt::ZERO),
    };

    let pool_price = core.get_pool_price(&key).call().await.unwrap();

    // let step = 0.5_f64.powi(64);
    // let x: f64 = step * pool_price.sqrt_ratio.low;
    // let zzz: f64 = u64::try_from(pool_price.sqrt_ratio.high).unwrap() as f64 + ;

    println!("Pool price: {:?}", pool_price);

    let swapper_address = deploy_swapper(&client, &account).await;
    let swapper = contracts_swapper::ContractsSwapper::new(swapper_address, &account);

    client
        .top_up_balance(swapper_address, 1000_u32, TokenId::ETH)
        .await
        .unwrap();

    client
        .top_up_balance(account.address(), 1_u32, TokenId::USDC)
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

    let swap_one_eth_for_usdc = SwapData {
        pool_key: contracts_swapper::PoolKey {
            token0: cainome::cairo_serde::ContractAddress::from(eth_token.l2_token_address),
            token1: cainome::cairo_serde::ContractAddress::from(usdc_token.l2_token_address),
            fee: 170141183460469235273462165868118016,
            tick_spacing: 1000,
            extension: cainome::cairo_serde::ContractAddress::from(Felt::ZERO),
        },
        sqrt_ratio_limit: U256 { low: 0, high: 3000 }, // actually sqrt(token1/token0) so sqrt(3000) would be enough, but I don't want to calculate fp value here
        amount: I129 {
            mag: 1,
            sign: false,
        },
        token: cainome::cairo_serde::ContractAddress::from(eth_token.l2_token_address),
    };

    let usdc_erc20 = Erc20MintableOZ0::new(usdc_token.l2_token_address, &account);

    client.create_block().await.unwrap();

    let balance_before = usdc_erc20
        .balanceOf(&cainome::cairo_serde::ContractAddress::from(
            account.address(),
        ))
        .call()
        .await
        .unwrap();

    println!("USDC balance before swap: {:?}", balance_before);

    let res = swapper.swap(&swap_one_eth_for_usdc).send().await;
    if let Err(e) = res {
        println!("Swap failed: {:?}", e);
        return;
    }

    client.create_block().await.unwrap();

    let balance_after = usdc_erc20
        .balanceOf(&cainome::cairo_serde::ContractAddress::from(
            account.address(),
        ))
        .call()
        .await
        .unwrap();

    println!("USDC balance after swap: {:?}", balance_after);

    // To swap I need.
    // Fund swapper contract
    // Exeute swap
    // Check my balance.

    // let swapper = contracts_swapper::ContractsSwapper::new(swapper_address, &account);

    env.stop();
}
