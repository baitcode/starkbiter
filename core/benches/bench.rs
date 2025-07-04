//! Benchmarking Starkbiter contracts. Note: Heavy WIP.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use cainome::cairo_serde::{ContractAddress, U256};
use starkbiter_bindings::erc_20_mintable_oz0::Erc20MintableOZ0;
use starkbiter_core::{
    environment::Environment,
    middleware::{traits::Middleware, StarkbiterMiddleware},
};
use starknet_accounts::ConnectedAccount;
use starknet_core::{
    types::{Call, Felt},
    utils::get_selector_from_name,
};
use starknet_devnet_core::constants::{self, ARGENT_CONTRACT_CLASS_HASH};
use starknet_signers::SigningKey;
use tracing::info;

const NUM_BENCH_ITERATIONS: usize = 100;
const NUM_LOOP_STEPS: usize = 10;

#[derive(Debug)]
struct BenchDurations {
    deploy: Duration,
    lookup: Duration,
    stateless_call: Duration,
    stateful_call: Duration,
}

#[tokio::main]
async fn main() {
    // Choose the benchmark group items by label.
    let group = ["arbiter"];
    let mut results: HashMap<&str, HashMap<&str, Duration>> = HashMap::new();

    // Set up for showing percentage done.
    let ten_percent = NUM_BENCH_ITERATIONS / 10;

    for item in group {
        let mut item_results = HashMap::new();
        // Count up total durations for each part of the benchmark.
        let mut durations = Vec::with_capacity(NUM_BENCH_ITERATIONS);
        println!("Running {item} benchmark");

        for index in 0..NUM_BENCH_ITERATIONS {
            durations.push(match item {
                label @ "starkbiter" => {
                    let (_environment, client) = starkbiter_startup();
                    bencher(client, label).await
                }
                _ => panic!("Invalid argument"),
            });
            if index % ten_percent == 0 {
                println!("{index} out of {NUM_BENCH_ITERATIONS} complete");
            }
        }
        let sum_durations = durations.iter().fold(
            BenchDurations {
                deploy: Duration::default(),
                lookup: Duration::default(),
                stateless_call: Duration::default(),
                stateful_call: Duration::default(),
            },
            |acc, duration| BenchDurations {
                deploy: acc.deploy + duration.deploy,
                lookup: acc.lookup + duration.lookup,
                stateless_call: acc.stateless_call + duration.stateless_call,
                stateful_call: acc.stateful_call + duration.stateful_call,
            },
        );

        let average_durations = BenchDurations {
            deploy: sum_durations.deploy / NUM_BENCH_ITERATIONS as u32,
            lookup: sum_durations.lookup / NUM_BENCH_ITERATIONS as u32,
            stateless_call: sum_durations.stateless_call / NUM_BENCH_ITERATIONS as u32,
            stateful_call: sum_durations.stateful_call / NUM_BENCH_ITERATIONS as u32,
        };

        item_results.insert("Deploy", average_durations.deploy);
        item_results.insert("Lookup", average_durations.lookup);
        item_results.insert("Stateless Call", average_durations.stateless_call);
        item_results.insert("Stateful Call", average_durations.stateful_call);

        results.insert(item, item_results);
    }

    // let df = create_dataframe(&results, &group);

    match get_version_of("starkbiter-core") {
        Some(version) => println!("starkbiter-core version: {}", version),
        None => println!("Could not find version for starkbiter-core"),
    }

    println!("Date: {}", chrono::Local::now().format("%Y-%m-%d"));
    // println!("{}", df);
}

async fn bencher(client: Arc<StarkbiterMiddleware>, label: &str) -> BenchDurations {
    let account = client
        .create_single_owner_account(
            Option::<SigningKey>::None,
            ARGENT_CONTRACT_CLASS_HASH,
            10000000,
        )
        .await
        .unwrap();

    // Track the duration for each part of the benchmark.
    let mut total_deploy_duration = 0;
    let mut total_lookup_duration = 0;
    let total_stateless_call_duration = 0;
    let mut total_stateful_call_duration = 0;

    // Deploy `ArbiterMath` and `ArbiterToken` contracts and tally up how long this
    // takes.
    let (arbiter_token, deploy_duration) = deployments(client.clone(), &account, label).await;
    total_deploy_duration += deploy_duration.as_micros();

    // Call `balance_of` `NUM_LOOP_STEPS` times on `ArbiterToken` and tally up how
    // long basic lookups take.
    let lookup_duration = lookup(&account, &arbiter_token, label).await;
    total_lookup_duration += lookup_duration.as_micros();

    // Call `cdf` `NUM_LOOP_STEPS` times on `ArbiterMath` and tally up how long this
    // takes.
    // let stateless_call_duration = stateless_call_loop(arbiter_math, label).await;
    // total_stateless_call_duration += stateless_call_duration.as_micros();

    // Call `mint` `NUM_LOOP_STEPS` times on `ArbiterToken` and tally up how long
    // this takes.
    let statefull_call_duration = stateful_call_loop(&account, &arbiter_token, label).await;
    total_stateful_call_duration += statefull_call_duration.as_micros();

    BenchDurations {
        deploy: Duration::from_micros(total_deploy_duration as u64),
        lookup: Duration::from_micros(total_lookup_duration as u64),
        stateless_call: Duration::from_micros(total_stateless_call_duration as u64),
        stateful_call: Duration::from_micros(total_stateful_call_duration as u64),
    }
}

fn starkbiter_startup() -> (Environment, Arc<StarkbiterMiddleware>) {
    let environment = Environment::builder().build();

    let client = StarkbiterMiddleware::new(&environment, Some("name")).unwrap();
    (environment, client)
}

async fn deployments<A: ConnectedAccount + Sync + Clone>(
    client: Arc<StarkbiterMiddleware>,
    account: &A,
    label: &str,
) -> (Erc20MintableOZ0<A>, Duration) {
    let start = Instant::now();

    let deploy_call = vec![Call {
        to: constants::UDC_CONTRACT_ADDRESS,
        selector: get_selector_from_name("deployContract").unwrap(),
        calldata: vec![
            constants::CAIRO_1_ERC20_CONTRACT_CLASS_HASH, // class hash
            Felt::from_hex_unchecked("0x123"),            // salt
            Felt::ZERO,                                   // unique
            Felt::ONE,                                    // constructor length
            account.address(),                            // constructor arguments
        ],
    }];

    let result = account.execute_v3(deploy_call).send().await.unwrap();

    let address = client
        .get_deployed_contract_address(result.transaction_hash)
        .await
        .unwrap();

    let token = Erc20MintableOZ0::new(address, account.clone());

    let duration = start.elapsed();
    info!("Time elapsed in {} deployment is: {:?}", label, duration);

    (token, duration)
}

async fn lookup<A: ConnectedAccount + Sync + Clone>(
    account: &A,
    arbiter_token: &Erc20MintableOZ0<A>,
    label: &str,
) -> Duration {
    let start = Instant::now();
    for _ in 0..NUM_LOOP_STEPS {
        arbiter_token
            .balanceOf(&ContractAddress::from(account.address()))
            .call()
            .await
            .unwrap();
    }
    let duration = start.elapsed();
    info!("Time elapsed in {} cdf loop is: {:?}", label, duration);

    duration
}

// async fn stateless_call_loop<A: ConnectedAccount + Sync + Clone>(
//     account: &A,
//     arbiter_token: Erc20MintableOZ0<A>,
//     label: &str,
// ) -> Duration {
//     let iwad = I256::from(10_u128.pow(18));
//     let start = Instant::now();
//     for _ in 0..NUM_LOOP_STEPS {
//         arbiter_math.cdf(iwad).call().await.unwrap();
//     }
//     let duration = start.elapsed();
//     info!("Time elapsed in {} cdf loop is: {:?}", label, duration);

//     duration
// }

async fn stateful_call_loop<A: ConnectedAccount + Sync + Clone>(
    account: &A,
    arbiter_token: &Erc20MintableOZ0<A>,
    label: &str,
) -> Duration {
    let wad = U256 {
        low: 10_u128.pow(18),
        high: 0,
    };
    let start = Instant::now();
    for _ in 0..NUM_LOOP_STEPS {
        arbiter_token
            .mint(&ContractAddress::from(account.address()), &wad)
            .send()
            .await
            .unwrap();
    }
    let duration = start.elapsed();
    info!("Time elapsed in {} mint loop is: {:?}", label, duration);

    duration
}

// fn create_dataframe(results: &HashMap<&str, HashMap<&str, Duration>>, group:
// &[&str]) -> DataFrame {     let operations = ["Deploy", "Lookup", "Stateless
// Call", "Stateful Call"];     let mut df = DataFrame::new(vec![
//         Series::new("Operation", operations.to_vec()),
//         Series::new(
//             &format!("{} (μs)", group[0]),
//             operations
//                 .iter()
//                 .map(|&op|
// results.get(group[0]).unwrap().get(op).unwrap().as_micros() as f64)
//                 .collect::<Vec<_>>(),
//         ),
//         Series::new(
//             &format!("{} (μs)", group[1]),
//             operations
//                 .iter()
//                 .map(|&op|
// results.get(group[1]).unwrap().get(op).unwrap().as_micros() as f64)
//                 .collect::<Vec<_>>(),
//         ),
//     ])
//     .unwrap();

//     let s0 = df.column(&format!("{} (μs)", group[0])).unwrap().to_owned();
//     let s1 = df.column(&format!("{} (μs)", group[1])).unwrap().to_owned();
//     let mut relative_difference = s0.divide(&s1).unwrap();

//     df.with_column::<Series>(relative_difference.rename("Relative
// Speedup").clone())         .unwrap()
//         .clone()
// }

fn get_version_of(crate_name: &str) -> Option<String> {
    let metadata = cargo_metadata::MetadataCommand::new().exec().unwrap();

    for package in metadata.packages {
        if package.name == crate_name {
            return Some(package.version.to_string());
        }
    }

    None
}
