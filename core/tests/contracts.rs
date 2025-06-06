use std::str::FromStr;

use arbiter_bindings::erc_20_mintable_oz0::Erc20MintableOZ0;
use cainome::cairo_serde::U256;

include!("common.rs");

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_create_account_and_query_erc20_for_balance() {
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

    let client = ArbiterMiddleware::new(&env, Some("wow")).unwrap();
    let signing_key = SigningKey::from_random();
    let public_key = signing_key.verifying_key();

    let address = client
        .create_account(
            public_key,
            constants::ARGENT_CONTRACT_CLASS_HASH,
            100000_u128,
        )
        .await
        .unwrap();

    let account = SingleOwnerAccount::new(
        &client,
        LocalWallet::from_signing_key(signing_key),
        address.into(),
        MAINNET,
        ExecutionEncoding::New,
    );

    let erc20_strk_contract =
        Erc20MintableOZ0::new(constants::STRK_ERC20_CONTRACT_ADDRESS, account);

    let balance = erc20_strk_contract
        .balanceOf(&Felt::from(address).into())
        .call()
        .await
        .unwrap();

    assert!(
        balance == U256::from_str("100000").unwrap(),
        "Balance shoyld be 100000",
    );
}
