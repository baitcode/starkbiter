use std::num::NonZero;

use starkbiter_core::{
    environment::{instruction::EventFilter, Environment},
    middleware::{traits::Middleware, StarkbiterMiddleware},
};
use starknet::providers::sequencer::models::Block;
use starknet_core::{
    types::{
        BlockId, BlockTag, BroadcastedDeclareTransactionV3, BroadcastedDeployAccountTransactionV3,
        BroadcastedInvokeTransactionV3, ContractClass, DeclareTransaction,
        DeployAccountTransaction, Felt, InvokeTransaction, L1HandlerTransaction,
        MaybePendingBlockWithTxs,
    },
    utils::starknet_keccak,
};
use starknet_devnet_types::{
    chain_id::ChainId, rpc::gas_modification::GasModificationRequest,
    starknet_api::test_utils::deploy_account,
};
use url::Url;

const SOME_ADDRESS: &str = "0x00000005dd3D2F4429AF886cD1a3b08289DBcEa99A294197E9eB43b0e0325b4b";

// TODO: shall I test env separately?

#[tokio::test]
async fn test_set_chain_id() {
    // Custom chain ID for Starknet
    let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
    let chain_id = ChainId::Custom(chain_id_felt);

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    let internal_chain_id = client.chain_id().await.unwrap();

    assert!(
        internal_chain_id == chain_id_felt,
        "Chain ID does not match expected value"
    );

    let _ = env.stop();
}

#[tokio::test]
async fn test_create_block() {
    // Custom chain ID for Starknet
    let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
    let chain_id = ChainId::Custom(chain_id_felt);

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    let block_number = client.block_number().await.unwrap();
    client.create_block().await.unwrap();
    let new_block_number = client.block_number().await.unwrap();

    assert!(
        block_number == new_block_number - 1,
        "Block number did not increment as expected old: {}, new: {}",
        block_number,
        new_block_number
    );

    let _ = env.stop();
}

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
async fn test_set_next_block_gas() {
    // Custom chain ID for Starknet
    let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
    let chain_id = ChainId::Custom(chain_id_felt);

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    let block_number = client.block_number().await.unwrap();
    client.set_next_block_gas(ALL_GAS_1).await.unwrap();
    let new_block_number = client.block_number().await.unwrap();

    assert!(
        block_number == new_block_number - 1,
        "Block number did not increment as expected old: {}, new: {}",
        block_number,
        new_block_number
    );
    let maybe_block = client
        .get_block_with_txs(BlockId::Number(new_block_number))
        .await
        .unwrap();

    if let MaybePendingBlockWithTxs::Block(block) = maybe_block {
        assert!(block.l1_data_gas_price.price_in_wei == Felt::ONE);
        assert!(block.l1_data_gas_price.price_in_fri == Felt::ONE);

        assert!(block.l1_gas_price.price_in_wei == Felt::ONE);
        assert!(block.l1_gas_price.price_in_fri == Felt::ONE);

        assert!(block.l2_gas_price.price_in_wei == Felt::ONE);
        assert!(block.l2_gas_price.price_in_fri == Felt::ONE);
    } else {
        assert!(false, "Block should've been finalised")
    }

    let _ = env.stop();
}

#[tokio::test]
async fn test_set_get_storage_at() {
    // Custom chain ID for Starknet
    let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
    let chain_id = ChainId::Custom(chain_id_felt);

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    {
        let maybe_val = client
            .get_storage_at(
                Felt::from_hex_unchecked(SOME_ADDRESS),
                Felt::from_hex_unchecked("0x696e766f6b65"),
                BlockId::Tag(BlockTag::Latest),
            )
            .await;

        assert!(!maybe_val.is_err(), "Should not throw error ever");

        let val = maybe_val.unwrap();
        assert!(Felt::ZERO == val, "Value should be 0");
    }

    client
        .set_storage_at(
            Felt::from_hex_unchecked(SOME_ADDRESS),
            Felt::from_hex_unchecked("0x696e766f6b65"),
            Felt::from_hex_unchecked("0x1234567890abcdef"),
        )
        .await
        .expect("Should not throw error");

    client.create_block().await.expect("Should not throw error");

    {
        let maybe_val = client
            .get_storage_at(
                Felt::from_hex_unchecked(SOME_ADDRESS),
                Felt::from_hex_unchecked("0x696e766f6b65"),
                BlockId::Tag(BlockTag::Latest),
            )
            .await;

        assert!(!maybe_val.is_err(), "Should not throw error ever");

        let val = maybe_val.unwrap();
        assert!(
            Felt::from_hex_unchecked("0x1234567890abcdef") == val,
            "Value should be 0x1234567890abcdef, got: {}",
            val
        );
    };

    let _ = env.stop();
}

#[tokio::test]
async fn test_class_hash_at() {
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

    let class_hash = client
        .get_class_hash_at(
            BlockId::Tag(BlockTag::Latest),
            Felt::from_hex_unchecked(SOME_ADDRESS),
        )
        .await
        .expect("Should not throw error");

    assert!(
        class_hash
            == Felt::from_hex_unchecked(
                "0x02a6f52c3635211b907467b53e3a712be41f0115d03f9d5903d448a1cd4f6f5c"
            )
    );

    let _ = env.stop();
}

/// Skipped as devnet does not support class at for forked classes defined
/// outside of current instance.
#[allow(unused)]
async fn test_class_at() {
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

    let contract_class = client
        .get_class(
            BlockId::Tag(BlockTag::Latest),
            Felt::from_hex_unchecked(
                "0x02a6f52c3635211b907467b53e3a712be41f0115d03f9d5903d448a1cd4f6f5c",
            ),
        )
        .await
        .expect("Should not throw error");

    if let ContractClass::Sierra(_class) = contract_class {
        // compare with ekubo contract class implementation
    } else {
        assert!(
            false,
            "Expected a Sierra contract class, got: {:?}",
            contract_class
        );
    }
    // contract_class.
    let _ = env.stop();
}

// Custom chain ID for Starknet

// #[tokio::test]
// async fn test_set_block_timestamp() {
//     // Custom chain ID for Starknet
//     let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
//     let chain_id = ChainId::Custom(chain_id_felt);

//     // Spin up a new environment with the specified chain ID
//     let env = Environment::builder()
//         .with_chain_id(chain_id.into())
//         .build();

//     let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

//     client.set

//     let block_number = client.block_number().await.unwrap();
//     client.create_block().await.unwrap();
//     let new_block_number = client.block_number().await.unwrap();

//     assert!(
//         block_number == new_block_number - 1,
//         "Block number did not increment as expected old: {}, new: {}",
//         block_number,
//         new_block_number
//     );
// }

// TODO: add impersonation
// TODO: logging?

#[tokio::test]
async fn test_get_block_with_txs_for_future_blocks() {
    std::env::set_var("RUST_LOG", "debug");
    let _ = tracing_subscriber::fmt::try_init();

    // Custom chain ID for Starknet
    let chain_id_felt = Felt::from_hex("0x696e766f6b65").unwrap();
    let chain_id = ChainId::Custom(chain_id_felt);

    let alchemy_key = std::env::var("ALCHEMY_KEY").expect("ALCHEMY_KEY must be set");
    let url = Url::parse(&format!(
        "https://starknet-mainnet.g.alchemy.com/starknet/version/rpc/v0_8/{}/",
        alchemy_key
    ))
    .unwrap();

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .with_fork(
            url,
            0,
            Felt::from_hex_unchecked(
                "0x47c3637b57c2b079b93c61539950c17e868a28f46cdef28f88521067f21e943",
            ),
        )
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    let block = client
        .get_block_with_txs_from_fork(BlockId::Number(1558913))
        .await
        .expect("Block was not found should've been");

    assert!(
        block.transactions().len() == 194,
        "Expected 14 transactions in block 6, got: {}",
        block.transactions().len()
    );
}

#[tokio::test]
async fn test_replay_block_transactions_containing_ekubo_swaps() {
    std::env::set_var(
        "RUST_LOG",
        "starkbiter_core=trace,starknet_devnet_core=debug,starknet_devnet_core::starknet::defaulter=info,blockifier=debug",
    );
    // std::env::set_var("RUST_LOG", "trace");
    let _ = tracing_subscriber::fmt::try_init();

    // Custom chain ID for Starknet
    let chain_id = ChainId::Mainnet;

    let alchemy_key = std::env::var("ALCHEMY_KEY").expect("ALCHEMY_KEY must be set");
    let node_url = format!(
        "https://starknet-mainnet.g.alchemy.com/starknet/version/rpc/v0_8/{}/",
        alchemy_key
    );
    let url = Url::parse(&node_url).unwrap();

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .with_fork(
            url.clone(),
            1586288,
            Felt::from_hex_unchecked(
                "0x634060800585f64b2f5c51cfd14f2770057b7a28ab39767558159e8037acd38",
            ),
        )
        .build();

    let client = StarkbiterMiddleware::new(&env, Some("wow")).unwrap();

    let filter = EventFilter {
        // Ekubo: Core
        from_address: Felt::from_hex_unchecked(
            "0x00000005dd3d2f4429af886cd1a3b08289dbcea99a294197e9eb43b0e0325b4b",
        ),
        // Selector for: Swapped
        keys: vec![vec![Felt::from_hex_unchecked(
            "0x157717768aca88da4ac4279765f09f4d0151823d573537fbbeb950cdbd9a870",
        )]],
    };

    let (added, ignored, failed) = client
        .replay_block_with_txs(url, BlockId::Number(1586289), Some(vec![filter]), false)
        .await
        .expect("Block was not found should've been");

    assert!(
        added > 0,
        "Expected to add some transactions, got: {}",
        added
    );
}

// TODO: add impersonation
// TODO: logging?
