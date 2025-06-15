use arbiter_bindings::{
    contracts_counter::ContractsCounter, erc_20_mintable_oz0::Erc20MintableOZ0, ARGENT_v040_SIERRA,
    ERC20_CONTRACT_SIERRA,
};
use arbiter_core::middleware::traits::Middleware;
use starknet_accounts::Account;
use std::{num::NonZero, str::FromStr};

use cainome::cairo_serde::{ContractAddress, U256};

use starknet_core::{types::Call, utils::get_selector_from_name};

use starknet_devnet_types::{chain_id::ChainId, rpc::gas_modification::GasModificationRequest};

include!("common.rs");

const ALL_GAS_1: GasModificationRequest = GasModificationRequest {
    gas_price_wei: NonZero::new(1_u128),
    gas_price_fri: NonZero::new(1_u128),

    data_gas_price_wei: NonZero::new(1_u128),
    data_gas_price_fri: NonZero::new(1_u128),

    l2_gas_price_wei: NonZero::new(1_u128),
    l2_gas_price_fri: NonZero::new(1_u128),

    generate_block: Some(true),
};

#[tokio::test]
async fn test_create_account_and_use_it_to_deploy_udc_counter_contract() {
    log();

    // Custom chain ID for Starknet
    let chain_id = ChainId::Custom(Felt::from_str("0x696e766f6b65").unwrap());

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = ArbiterMiddleware::new(&env, Some("wow")).unwrap();

    let argent_class_hash = client.declare_contract(ARGENT_v040_SIERRA).await.unwrap();

    client.set_next_block_gas(ALL_GAS_1).await.unwrap();
    client.create_block().await.unwrap();

    let account = client
        .create_single_owner_account(Option::<SigningKey>::None, argent_class_hash, 1000000000)
        .await
        .unwrap();

    let counter_class_hash = client
        .declare_contract(arbiter_bindings::COUNTER_CONTRACT_SIERRA)
        .await
        .unwrap();

    let deploy_call = vec![Call {
        to: constants::UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            counter_class_hash,                // class hash
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,                        // unique
            Felt::ZERO,                        // constructor length
        ],
    }];

    let result = account.execute_v3(deploy_call).send().await.unwrap();

    let address = client
        .get_deployed_contract_address(result.transaction_hash)
        .await
        .unwrap();

    tracing::info!("Deployed contract address: {:?}", address);

    let counter_contract = ContractsCounter::new(address, account);

    let result = counter_contract
        .get(&ContractAddress::from(Felt::from_hex("0x0").unwrap()))
        .call()
        .await
        .unwrap();

    assert!(result == 0, "Counter should be zero");

    let _ = counter_contract.increment().send().await.unwrap();

    let result = counter_contract
        .get(&ContractAddress::from(Felt::from_hex("0x0").unwrap()))
        .call()
        .await
        .unwrap();

    assert!(result == 1, "Counter should be one");
}

#[tokio::test]
async fn test_create_account_and_use_it_to_deploy_udc_erc20_contract() {
    log();

    // Custom chain ID for Starknet
    let chain_id = ChainId::Custom(Felt::from_str("0x696e766f6b65").unwrap());

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = ArbiterMiddleware::new(&env, Some("wow")).unwrap();

    let argent_class_hash = client.declare_contract(ARGENT_v040_SIERRA).await.unwrap();

    client.set_next_block_gas(ALL_GAS_1).await.unwrap();
    client.create_block().await.unwrap();

    let account = client
        .create_single_owner_account(Option::<SigningKey>::None, argent_class_hash, 1000000000)
        .await
        .unwrap();

    let account_address = account.address();

    let erc20_class_hash = client
        .declare_contract(ERC20_CONTRACT_SIERRA)
        .await
        .unwrap();

    let deploy_call = vec![Call {
        to: constants::UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            erc20_class_hash,                  // class hash
            Felt::from_hex_unchecked("0x123"), // salt
            Felt::ZERO,                        // unique
            Felt::ONE,                         // constructor length
            account_address,                   // constructor arguments
        ],
    }];

    let result = account.execute_v3(deploy_call).send().await.unwrap();

    let address = client
        .get_deployed_contract_address(result.transaction_hash)
        .await
        .unwrap();

    tracing::info!("Deployed contract address: {:?}", address);

    let erc20 = Erc20MintableOZ0::new(address, account);

    let balance = erc20
        .balanceOf(&cainome::cairo_serde::ContractAddress::from(
            account_address,
        ))
        .call()
        .await
        .unwrap();

    let total_supply = erc20.totalSupply().call().await.unwrap();

    assert!(
        balance == U256::from_str("0").unwrap(),
        "Balance should be 0"
    );

    assert!(
        total_supply == U256::from_str("0").unwrap(),
        "Balance should be 0"
    );

    let a_hundred = U256::from_str("100").unwrap();

    erc20
        .mint(
            &cainome::cairo_serde::ContractAddress::from(account_address),
            &a_hundred,
        )
        .send()
        .await
        .unwrap();

    let balance = erc20
        .balanceOf(&cainome::cairo_serde::ContractAddress::from(
            account_address,
        ))
        .call()
        .await
        .unwrap();

    let total_supply = erc20.totalSupply().call().await.unwrap();

    assert!(balance == a_hundred, "Balance should be 100");

    assert!(total_supply == a_hundred, "Balance should be 100");
}
