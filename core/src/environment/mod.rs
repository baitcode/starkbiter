//! The [`environment`] module provides abstractions and functionality for
//! handling the Ethereum execution environment. This includes managing its
//! state, interfacing with the EVM, and broadcasting events to subscribers.
//! Other features include the ability to control block rate and gas settings
//! and execute other database modifications from external agents.
//!
//! The key integration for the environment is the Rust EVM [`revm`](https://github.com/bluealloy/revm).
//! This is an implementation of the EVM in Rust that we utilize for processing
//! raw smart contract bytecode.
//!
//! Core structures:
//! - [`Environment`]: Represents the Ethereum execution environment, allowing
//!   for its management (e.g., starting, stopping) and interfacing with agents.
//! - [`EnvironmentParameters`]: Parameters necessary for creating or modifying
//!   an [`Environment`].
//! - [`Instruction`]: Enum indicating the type of instruction that is being
//!   sent to the EVM.

use std::thread::{self, JoinHandle};

use crossbeam_channel::{unbounded, Receiver, Sender};
use starknet::providers::Url;
use starknet_core::types::{self as core_types, Call, EmittedEvent, Felt};
use starknet_core::utils::get_selector_from_name;
use starknet_devnet_core::constants::{self as devnet_constants};

use starknet_devnet_core::error::Error as DevnetError;
use starknet_devnet_core::starknet::starknet_config::ForkConfig;
use starknet_devnet_core::state::{CustomState, CustomStateReader};
use starknet_devnet_core::{
    starknet::{starknet_config::StarknetConfig, Starknet},
    state::StateReader,
};
use starknet_devnet_types::chain_id::ChainId;
use starknet_devnet_types::contract_class::ContractClass;

use starknet_devnet_types::error::Error;
use starknet_devnet_types::rpc::block::BlockResult;
use starknet_devnet_types::rpc::gas_modification::GasModificationRequest;
use starknet_devnet_types::rpc::transaction_receipt::TransactionReceipt;
use starknet_devnet_types::rpc::transactions::broadcasted_deploy_account_transaction_v3::BroadcastedDeployAccountTransactionV3;
use starknet_devnet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, BroadcastedTransaction, SimulationFlag,
};
use starknet_devnet_types::starknet_api;
use starknet_devnet_types::starknet_api::transaction::fields::{Calldata, ContractAddressSalt};
use starknet_devnet_types::starknet_api::transaction::TransactionHasher;
use starknet_devnet_types::traits::HashProducer;
use starknet_devnet_types::{contract_address::ContractAddress, num_bigint::BigUint};

use starknet_devnet_types::starknet_api::core::{self as api_core, PatriciaKey};
use starknet_devnet_types::starknet_api::state::StorageKey;

use tokio::sync::broadcast::channel;

use crate::tokens::get_token_data;

use super::*;

pub mod instruction;
use instruction::{Instruction, NodeInstruction, NodeOutcome, Outcome, ReceiptData};

mod utils;

/// Alias for the sender of the channel for transmitting transactions.
pub(crate) type InstructionSender = Sender<(Instruction, OutcomeSender)>;

/// Alias for the receiver of the channel for transmitting transactions.
pub(crate) type InstructionReceiver = Receiver<(Instruction, OutcomeSender)>;

/// Alias for the sender of the channel for transmitting [`RevmResult`] emitted
/// from transactions.
pub(crate) type OutcomeSender = Sender<Result<Outcome, ArbiterCoreError>>;

/// Alias for the receiver of the channel for transmitting [`RevmResult`]
/// emitted from transactions.
pub(crate) type OutcomeReceiver = Receiver<Result<Outcome, ArbiterCoreError>>;

/// Represents a sandboxed EVM environment.
///
/// ## Features
/// * [`revm::Evm`] and its connections to the "outside world" (agents) via the
///   [`Socket`] provide the [`Environment`] a means to route and execute
///   transactions.
/// * [`ArbiterDB`] is the database structure used that allows for read-only
///   sharing of execution and write-only via the main thread. This can also be
///   a database read in from disk storage via [`database::fork::Fork`].
/// * [`ArbiterInspector`] is an that allows for the EVM to be able to display
///   logs and properly handle gas payments.
/// * [`EnvironmentParameters`] are used to set the gas limit, contract size
///   limit, and label for the [`Environment`].
#[derive(Debug)]
pub struct Environment {
    /// The label used to define the [`Environment`].
    pub parameters: EnvironmentParameters,

    /// The [`EVM`] that is used as an execution environment and database for
    /// calls and transactions.
    // pub(crate) db: ArbiterDB,
    // inspector: Option<ArbiterInspector>,

    /// This gives a means of letting the "outside world" connect to the
    /// [`Environment`] so that users (or agents) may send and receive data from
    /// the [`EVM`].
    pub(crate) socket: Socket,

    /// [`JoinHandle`] for the thread in which the [`EVM`] is running.
    /// Used for assuring that the environment is stopped properly or for
    /// performing any blocking action the end user needs.
    pub(crate) handle: Option<JoinHandle<Result<(), ArbiterCoreError>>>,
}

/// Parameters to create [`Environment`]s with different settings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EnvironmentParameters {
    pub chain_id: Option<ChainId>,

    /// The label used to define the [`Environment`].
    pub label: Option<String>,

    /// The gas limit for the blocks in the [`Environment`].
    pub gas_limit: Option<BigUint>,

    /// The contract size limit for the [`Environment`].
    pub contract_size_limit: Option<usize>,

    /// The URL of JSON RPC node endpoing to fork the Starknet network from.
    pub starknet_fork_url: Option<String>,

    /// The block number to fork the Starknet network from. Should be specified with starknet_fork_block_hash.
    pub starknet_fork_block_number: Option<u64>,

    /// The block hash to fork the Starknet network from. Should be specified with starknet_fork_block_number.
    pub starknet_fork_block_hash: Option<core_types::Felt>,

    /// Enables inner contract logs to be printed to the console.
    pub console_logs: bool,

    /// Allows for turning off any gas payments for transactions so no inspector
    /// is needed.
    pub pay_gas: bool,
}

/// A builder for creating an [`Environment`].
///
/// This builder allows for the configuration of an [`Environment`] before it is
/// instantiated. It provides methods for setting the label, gas limit, contract
/// size limit, and a database for the [`Environment`].
pub struct EnvironmentBuilder {
    parameters: EnvironmentParameters,
}

impl EnvironmentBuilder {
    /// Builds and runs an [`Environment`] with the parameters set in the
    /// [`EnvironmentBuilder`].
    pub fn build(self) -> Environment {
        Environment::create(
            self.parameters,
            // self.db,
        )
        .run()
    }

    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        self.parameters.chain_id = Some(chain_id);
        self
    }

    /// Sets the label for the [`Environment`].
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.parameters.label = Some(label.into());
        self
    }

    /// Sets fork url and block for the [`Environment`].
    /// Important: this does not support forking by tag.
    pub fn with_fork(mut self, url: Url, block_number: u64, block_hash: core_types::Felt) -> Self {
        self.parameters.starknet_fork_url = Some(url.to_string());
        self.parameters.starknet_fork_block_number = Some(block_number);
        self.parameters.starknet_fork_block_hash = Some(block_hash);
        self
    }
}

impl Environment {
    /// Creates a new [`EnvironmentBuilder`] with default parameters that can be
    /// used to build an [`Environment`].
    pub fn builder() -> EnvironmentBuilder {
        EnvironmentBuilder {
            parameters: EnvironmentParameters::default(),
        }
    }

    fn create(parameters: EnvironmentParameters, // , db: ArbiterDB
    ) -> Self {
        let (instruction_sender, instruction_receiver) = unbounded();
        let (event_broadcaster, _) = channel(512);
        let socket = Socket {
            instruction_sender: Arc::new(instruction_sender),
            instruction_receiver,
            event_broadcaster,
        };

        Self {
            socket,
            // inspector,
            parameters,
            // db,
            handle: None,
        }
    }

    /// This starts the [`Environment`] thread to process any [`Instruction`]s
    /// coming through the [`Socket`].
    fn run(mut self) -> Self {
        // Bring in parameters for the `Environment`.
        let label = self.parameters.label.clone();

        // Bring in the EVM db and log storage by cloning the interior Arc
        // (lightweight).
        // let db = self.db.clone();

        // Bring in the inspector
        // let inspector = self.inspector.take().unwrap();

        // Pull communication clones to move into a new thread.
        let instruction_receiver: Receiver<(
            Instruction,
            Sender<Result<Outcome, ArbiterCoreError>>,
        )> = self.socket.instruction_receiver.clone();

        let event_broadcaster = self.socket.event_broadcaster.clone();

        // let parameters = self.parameters.clone();

        let (fork_config, predeploy_erc20) =
            if let Some(url_str) = self.parameters.starknet_fork_url.clone() {
                (
                    ForkConfig {
                        url: url_str.parse().ok(),
                        block_number: Some(self.parameters.starknet_fork_block_number.unwrap()),
                        block_hash: Some(self.parameters.starknet_fork_block_hash.unwrap()),
                    },
                    true,
                )
                // (ForkConfig::default(), true)
            } else {
                (ForkConfig::default(), true)
            };

        // Move the EVM and its socket to a new thread and retrieve this handle
        let handle = thread::spawn(move || {
            let result = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    // let mut evm = Evm::new(db, inspector, parameters.clone());
                    let starknet_config = &StarknetConfig {
                        fork_config,
                        // predeploy_erc20,
                        chain_id: self.parameters.chain_id.unwrap_or(ChainId::Testnet),
                        ..StarknetConfig::default()
                    };

                    // TODO: support forking
                    // TODO: Simulated block production

                    // let mut env = Env::default();
                    // env.cfg.limit_contract_code_size = self.parameters.contract_size_limit;
                    // env.block.gas_limit = self.parameters.gas_limit.unwrap_or(eU256::MAX);

                    process_instructions(
                        starknet_config,
                        label.unwrap_or_else(|| "default".to_string()),
                        instruction_receiver,
                        event_broadcaster,
                    )
                    .await
                });

            Ok(result?)
        });

        self.handle = Some(handle);
        self
    }

    /// Stops the execution of the environment and returns the [`ArbiterDB`] in
    /// its final state.
    // pub fn stop(mut self) -> Result<ArbiterDB, ArbiterCoreError> {
    pub fn stop(mut self) -> Result<(), ArbiterCoreError> {
        todo!("");

        // let (outcome_sender, outcome_receiver) = bounded(1);

        // self.socket
        //     .instruction_sender
        //     .send(Instruction::Stop(outcome_sender))?;

        // let outcome = outcome_receiver.recv()??;

        // let db = match outcome {
        //     // Outcome::StopCompleted(stopped_db) => stopped_db,
        //     Outcome::StopCompleted() => (),
        //     _ => unreachable!(),
        // };

        // if let Some(label) = &self.parameters.label {
        //     warn!("Stopped environment with label: {}", label);
        // } else {
        //     warn!("Stopped environment with no label.");
        // }
        // drop(self.socket.instruction_sender);
        // self.handle
        //     .take()
        //     .unwrap()
        //     .join()
        //     .map_err(|_| ArbiterCoreError::JoinError)??;
        // Ok(db)
    }
}

/// Provides channels for communication between the EVM and external entities.
///
/// The socket contains senders and receivers for transactions, as well as an
/// event broadcaster to broadcast logs from the EVM to subscribers.
#[derive(Debug, Clone)]
pub(crate) struct Socket {
    pub(crate) instruction_sender: Arc<InstructionSender>,
    pub(crate) instruction_receiver: InstructionReceiver,
    pub(crate) event_broadcaster: BroadcastSender<Vec<core_types::EmittedEvent>>,
}

async fn process_instructions(
    starknet_config: &StarknetConfig,
    label: String,
    instruction_receiver: Receiver<(Instruction, Sender<Result<Outcome, ArbiterCoreError>>)>,
    event_broadcaster: BroadcastSender<Vec<core_types::EmittedEvent>>,
) -> Result<(), ArbiterCoreError> {
    trace!(
        "Forking url: {:?} at blockHash {:?} and block number: {:?}. Erc20 predeploy: {:?}",
        starknet_config.fork_config.url,
        starknet_config.fork_config.block_hash,
        starknet_config.fork_config.block_number,
        false
    );

    // Fork configuration
    let mut starknet = Starknet::new(&starknet_config).unwrap();

    trace!("Devnet created");
    // Initialize counters that are returned on some receipts.
    // Loop over the instructions sent through the socket.
    while let Ok((instruction, sender)) = instruction_receiver.recv() {
        let result: Result<Outcome, ArbiterCoreError> = match instruction {
            Instruction::Node(ref basic_instruction) => match basic_instruction {
                NodeInstruction::GetSpecVersion => {
                    trace!("Environment. Received GetSpecVersion instruction");
                    let outcome = Outcome::Node(NodeOutcome::SpecVersion("unknown".to_string()));
                    Ok(outcome)
                }
                NodeInstruction::GetBlockWithTxHashes { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithTxHashes instruction: {:?}",
                        block_id
                    );
                    let block_result = starknet
                        .get_block_with_transactions(block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let outcome: core_types::MaybePendingBlockWithTxHashes = match block_result {
                        BlockResult::PendingBlock(block) => {
                            core_types::MaybePendingBlockWithTxHashes::PendingBlock(
                                core_types::PendingBlockWithTxHashes::from(block),
                            )
                        }
                        BlockResult::Block(block) => {
                            core_types::MaybePendingBlockWithTxHashes::Block(
                                core_types::BlockWithTxHashes::from(block),
                            )
                        }
                    };

                    Ok(Outcome::Node(NodeOutcome::GetBlockWithTxHashes(outcome)))
                }
                NodeInstruction::GetBlockWithTxs { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithTxs instruction: {:?}",
                        block_id
                    );
                    let block_result = starknet
                        .get_block_with_transactions(&block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let outcome: core_types::MaybePendingBlockWithTxs = match block_result {
                        BlockResult::PendingBlock(block) => {
                            core_types::MaybePendingBlockWithTxs::PendingBlock(
                                core_types::PendingBlockWithTxs::from(block),
                            )
                        }
                        BlockResult::Block(block) => core_types::MaybePendingBlockWithTxs::Block(
                            core_types::BlockWithTxs::from(block),
                        ),
                    };

                    Ok(Outcome::Node(NodeOutcome::GetBlockWithTxs(outcome)))
                }
                NodeInstruction::GetBlockWithReceipts { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithReceipts instruction: {:?}",
                        block_id
                    );
                    let block_result = starknet
                        .get_block_with_receipts(&block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let outcome: core_types::MaybePendingBlockWithReceipts = match block_result {
                        BlockResult::PendingBlock(block) => {
                            core_types::MaybePendingBlockWithReceipts::PendingBlock(
                                core_types::PendingBlockWithReceipts::from(block),
                            )
                        }
                        BlockResult::Block(block) => {
                            core_types::MaybePendingBlockWithReceipts::Block(
                                core_types::BlockWithReceipts::from(block),
                            )
                        }
                    };

                    Ok(Outcome::Node(NodeOutcome::GetBlockWithReceipts(outcome)))
                }
                NodeInstruction::GetStateUpdate { block_id } => {
                    trace!(
                        "Environment. Received GetStateUpdate instruction: {:?}",
                        block_id
                    );
                    let res = starknet
                        .block_state_update(block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetStateUpdate(res.into())))
                }
                NodeInstruction::GetStorageAt {
                    contract_address,
                    key,
                    // TODO: hmmmm, something is off
                    block_id,
                } => {
                    trace!(
                        "Environment. Received GetStorageAt instruction: {:?} {:?} {:?}",
                        contract_address,
                        key,
                        block_id
                    );
                    let state = starknet.get_state();

                    let contract_address = api_core::ContractAddress::try_from(*contract_address)
                        .map_err(|e| {
                        ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e))
                    })?;

                    let storage_key = StorageKey::try_from(*key).map_err(|e| {
                        ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e))
                    })?;

                    let outcome = state
                        .get_storage_at(contract_address, storage_key)
                        .map_err(|e| {
                            ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e))
                        })?;

                    Ok(Outcome::Node(NodeOutcome::GetStorageAt(outcome)))
                }
                NodeInstruction::GetMessagesStatus { transaction_hash } => {
                    trace!(
                        "Environment. Received GetMessagesStatus instruction: {:?}",
                        transaction_hash
                    );
                    let outcome = starknet
                        .get_messages_status(*transaction_hash)
                        .unwrap_or(Vec::new());
                    Ok(Outcome::Node(NodeOutcome::GetMessagesStatus(
                        outcome.iter().map(Into::into).collect(),
                    )))
                }
                NodeInstruction::GetTransactionStatus { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionStatus instruction: {:?}",
                        transaction_hash
                    );
                    let transaction_hash = core_types::Felt::try_from(transaction_hash)
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    let outcome = starknet
                        .get_transaction_execution_and_finality_status(transaction_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetTransactionStatus(
                        outcome.into(),
                    )))
                }
                NodeInstruction::GetTransactionByHash { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionByHash instruction: {:?}",
                        transaction_hash
                    );
                    let transaction_hash = core_types::Felt::try_from(transaction_hash)
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    let transaction = starknet
                        .get_transaction_by_hash(transaction_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetTransactionByHash(
                        core_types::Transaction::from(transaction.clone()),
                    )))
                }
                NodeInstruction::GetTransactionByBlockIdAndIndex { block_id, index } => {
                    trace!(
                        "Environment. Received GetTransactionByBlockIdAndIndex instruction: {:?} {:?}",
                        block_id,
                        index
                    );
                    let transaction = starknet
                        .get_transaction_by_block_id_and_index(&block_id, *index)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetTransactionByBlockIdAndIndex(
                        core_types::Transaction::from(transaction.clone()),
                    )))
                }
                NodeInstruction::GetTransactionReceipt { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionReceipt instruction: {:?}",
                        transaction_hash
                    );
                    let transaction_hash = core_types::Felt::try_from(transaction_hash)
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    let receipt = starknet
                        .get_transaction_receipt_by_hash(&transaction_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetTransactionReceipt(
                        core_types::TransactionReceiptWithBlockInfo::from(receipt),
                    )))
                }
                NodeInstruction::GetClass {
                    block_id,
                    class_hash,
                } => {
                    trace!(
                        "Environment. Received GetClass instruction: {:?} {:?}",
                        block_id,
                        class_hash
                    );
                    let klass = starknet
                        .get_class(&block_id, *class_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetClass(
                        klass
                            .try_into()
                            .map_err(|e: Error| ArbiterCoreError::InternalError(e.to_string()))?,
                    )))
                }
                NodeInstruction::GetClassHashAt {
                    block_id,
                    contract_address,
                } => {
                    trace!(
                        "Environment. Received GetClassHashAt instruction: {:?} {:?}",
                        block_id,
                        contract_address
                    );

                    let contract_address =
                        ContractAddress::new(*contract_address).expect("Should always work");

                    let class_hash = starknet
                        .get_class_hash_at(&block_id, contract_address)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetClassHashAt(class_hash)))
                }
                NodeInstruction::GetClassAt {
                    block_id,
                    contract_address,
                } => {
                    trace!(
                        "Environment. Received GetClassAt instruction: {:?} {:?}",
                        block_id,
                        contract_address
                    );

                    let contract_address =
                        ContractAddress::new(*contract_address).expect("Should always work");

                    let contract_class = starknet
                        .get_class_at(&block_id, contract_address)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetClassAt(
                        contract_class
                            .try_into()
                            .map_err(|e: Error| ArbiterCoreError::InternalError(e.to_string()))?,
                    )))
                }
                NodeInstruction::GetBlockTransactionCount { block_id } => {
                    trace!(
                        "Environment. Received GetBlockTransactionCount instruction: {:?}",
                        block_id
                    );

                    let count = starknet
                        .get_block_txs_count(&block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetBlockTransactionCount(count)))
                }
                NodeInstruction::BlockNumber => {
                    trace!("Environment. Received BlockNumber instruction");

                    let block = starknet
                        .get_latest_block()
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::BlockNumber(
                        block.block_number().0,
                    )))
                }
                NodeInstruction::BlockHashAndNumber => {
                    trace!("Environment. Received BlockHashAndNumber instruction");

                    let block = starknet
                        .get_latest_block()
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::BlockHashAndNumber(
                        core_types::BlockHashAndNumber {
                            block_hash: block.block_hash(),
                            block_number: block.block_number().0,
                        },
                    )))
                }
                NodeInstruction::ChainId => {
                    trace!("Environment. Received ChainId instruction");

                    let chain_id = starknet_config.chain_id;
                    Ok(Outcome::Node(NodeOutcome::ChainId(chain_id.into())))
                }
                NodeInstruction::Syncing => Ok(Outcome::Node(NodeOutcome::Syncing(
                    core_types::SyncStatusType::NotSyncing,
                ))),
                NodeInstruction::GetEvents {
                    filter,
                    continuation_token,
                    chunk_size,
                } => {
                    trace!(
                        "Environment. Received GetEvents instruction: {:?} {:?} {:?}",
                        filter,
                        continuation_token,
                        chunk_size
                    );

                    let skip = if let Some(s) = continuation_token {
                        s.parse::<u64>()
                            .expect("Continuation token should be a valid u64")
                    } else {
                        0
                    };

                    let chunk_size: usize = if let Some(size) = chunk_size {
                        *size as usize
                    } else {
                        50_usize // TODO: Default chunk size. Move to config
                    };

                    let (events, has_filtered_events) = starknet
                        .get_events(
                            filter.from_block,
                            filter.to_block,
                            match filter.address {
                                Some(address) => {
                                    Some(ContractAddress::new(address).expect("Should always work"))
                                }
                                None => None,
                            },
                            filter.keys.clone(),
                            skip.try_into().expect("Skip should be a valid usize"),
                            Some(chunk_size),
                        )
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let continuation_token = if events.len() < chunk_size {
                        Option::None
                    } else {
                        Option::Some(format!("{}", skip + events.len() as u64))
                    };

                    let page = core_types::EventsPage {
                        events: events.iter().map(|e| e.into()).collect(),
                        continuation_token,
                    };

                    Ok(Outcome::Node(NodeOutcome::GetEvents(page)))
                }
                NodeInstruction::Call { request, block_id } => {
                    trace!(
                        "Environment. Received Call instruction: {:?} {:?}",
                        request,
                        block_id
                    );

                    let res = starknet
                        .call(
                            block_id,
                            request.contract_address,
                            request.entry_point_selector,
                            request.calldata.clone(),
                        )
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    trace!("Environment. Call res: {:?} ", res);

                    Ok(Outcome::Node(NodeOutcome::Call(res)))
                }
                NodeInstruction::AddInvokeTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddInvokeTransaction instruction: {:?}",
                        transaction
                    );

                    let converted_transaction =
                        BroadcastedInvokeTransaction::V3(transaction.clone().into());

                    let tx_hash = starknet
                        .add_invoke_transaction(converted_transaction)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::AddInvokeTransaction(
                        core_types::InvokeTransactionResult {
                            transaction_hash: tx_hash,
                        },
                    )))
                }
                NodeInstruction::AddDeclareTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddDeclareTransaction instruction: {:?}",
                        transaction
                    );

                    let (tx_hash, class_hash) = starknet
                        .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(
                            transaction.clone().into(),
                        )))
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::AddDeclareTransaction(
                        core_types::DeclareTransactionResult {
                            transaction_hash: tx_hash,
                            class_hash,
                        },
                    )))
                }
                NodeInstruction::AddDeployAccountTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddDeployAccountTransaction instruction: {:?}",
                        transaction
                    );

                    let (tx_hash, contract_address) = starknet
                        .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(
                            transaction.clone().into(),
                        ))
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::AddDeployAccountTransaction(
                        core_types::DeployAccountTransactionResult {
                            transaction_hash: tx_hash,
                            contract_address: core_types::Felt::from(contract_address),
                        },
                    )))
                }
                NodeInstruction::TraceTransaction { transaction_hash } => {
                    trace!(
                        "Environment. Received TraceTransaction instruction: {:?}",
                        transaction_hash
                    );

                    let trace = starknet
                        .get_transaction_trace_by_hash(*transaction_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::TraceTransaction(trace.into())))
                }
                NodeInstruction::SimulateTransactions {
                    block_id,
                    transactions,
                    simulation_flags,
                } => {
                    trace!(
                        "Environment. Received SimulateTransactions instruction: {:?} {:?} {:?}",
                        block_id,
                        transactions,
                        simulation_flags
                    );

                    let res = starknet
                        .simulate_transactions(
                            &block_id,
                            transactions
                                .iter()
                                .cloned()
                                .map(Into::into)
                                .collect::<Vec<BroadcastedTransaction>>()
                                .as_slice(),
                            simulation_flags.iter().cloned().map(Into::into).collect(),
                        )
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;
                    Ok(Outcome::Node(NodeOutcome::SimulateTransactions(
                        res.iter().cloned().map(Into::into).collect(),
                    )))
                }
                NodeInstruction::TraceBlockTransactions { block_id } => {
                    trace!(
                        "Environment. Received TraceBlockTransactions instruction: {:?}",
                        block_id
                    );

                    let res = starknet
                        .get_transaction_traces_from_block(&block_id)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::TraceBlockTransactions(
                        res.iter().cloned().map(Into::into).collect(),
                    )))
                }
                NodeInstruction::EstimateFee {
                    request,
                    simulate_flags,
                    block_id,
                } => {
                    trace!(
                        "Environment. Received EstimateFee instruction: {:?} {:?} {:?}",
                        request,
                        simulate_flags,
                        block_id
                    );

                    let txs = &[BroadcastedTransaction::from(request.clone())];

                    let simulation_flags = simulate_flags
                        .iter()
                        .map(|f| SimulationFlag::from(*f))
                        .collect::<Vec<_>>();

                    let fees = starknet
                        .estimate_fee(&block_id, txs, &simulation_flags.as_slice())
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::EstimateFee(
                        fees.iter().cloned().map(Into::into).collect(),
                    )))
                }
                NodeInstruction::EstimateMessageFee { message, block_id } => {
                    trace!(
                        "Environment. Received EstimateMessageFee instruction: {:?} {:?}",
                        message,
                        block_id
                    );

                    let fee = starknet
                        .estimate_message_fee(&block_id, message.clone())
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::EstimateMessageFee(fee.into())))
                }
                NodeInstruction::GetNonce {
                    block_id,
                    contract_address,
                } => {
                    trace!(
                        "Environment. Received GetNonce instruction: {:?} {:?}",
                        block_id,
                        contract_address
                    );

                    let contract_address =
                        ContractAddress::new(*contract_address).expect("Should always work.");

                    let nonce = starknet
                        .contract_nonce_at_block(&block_id, contract_address)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Node(NodeOutcome::GetNonce(nonce.into())))
                }
            },
            Instruction::Cheat(ref cheat_instruction) => match cheat_instruction {
                instruction::CheatInstruction::SetNextBlockGas { gas_modification } => {
                    trace!("Environment. Received SetNextBlockGas instruction");

                    let modification = starknet
                        .set_next_block_gas(gas_modification.clone())
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Cheat(
                        instruction::CheatcodesReturn::SetNextBlockGas(modification),
                    ))
                }
                instruction::CheatInstruction::DeclareContract { sierra_json } => {
                    trace!("Environment. Received DeclareContract instruction: ",);

                    let contract_class = ContractClass::cairo_1_from_sierra_json_str(&sierra_json)
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    let sierra_contract_class = ContractClass::Cairo1(contract_class);

                    let class_hash = sierra_contract_class
                        .generate_hash()
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    trace!("Class hash generated: {:?}", class_hash);

                    let state = starknet.get_state();

                    if !state.is_contract_declared(class_hash) {
                        state
                            .predeclare_contract_class(class_hash, sierra_contract_class.clone())?;
                        trace!("Contract class predeclared");
                    }

                    starknet
                        .commit_diff()
                        .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                    let class_hash = api_core::ClassHash(class_hash);

                    Ok(Outcome::Cheat(
                        instruction::CheatcodesReturn::DeclareContract(class_hash.0),
                    ))
                }
                instruction::CheatInstruction::CreateAccount {
                    signing_key,
                    class_hash,
                    prefunded_balance,
                } => {
                    trace!(
                        "Environment. Received CreateAccount instruction: {:?} {:?}",
                        class_hash,
                        prefunded_balance
                    );

                    let salt = core_types::Felt::from(20_u32);
                    let public_key = signing_key.verifying_key();
                    let calldata_felts = vec![
                        core_types::Felt::ZERO,
                        public_key.scalar(),
                        core_types::Felt::ONE,
                    ];

                    let account_address = api_core::calculate_contract_address(
                        ContractAddressSalt(salt), // TODO: rethink
                        api_core::ClassHash(*class_hash),
                        &Calldata(Arc::new(calldata_felts.clone())),
                        0_u128.into(),
                    )
                    .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                    let is_account_contract_declared = {
                        let state = starknet.get_state();
                        state.is_contract_declared(*class_hash)
                    };

                    {
                        let mut state = starknet.get_state();
                        // Same as Top up balance, except only strk token is minted
                        trace!("Minting tokens...");
                        utils::mint_tokens_in_erc20_contract(
                            &mut state,
                            devnet_constants::STRK_ERC20_CONTRACT_ADDRESS,
                            account_address.into(),
                            prefunded_balance.clone(),
                        )?;
                        starknet.commit_diff()?;
                    }

                    if !is_account_contract_declared {
                        Err(ArbiterCoreError::DevnetError(
                            DevnetError::ContractClassLoadError("Not declared".to_string()),
                        ))
                    } else {
                        let mut tx = starknet_api::transaction::DeployAccountTransactionV3 {
                            resource_bounds:
                                starknet_api::transaction::fields::ValidResourceBounds::AllResources(
                                    starknet_api::transaction::fields::AllResourceBounds {
                                        l1_gas: starknet_api::transaction::fields::ResourceBounds {
                                            max_amount:
                                                starknet_api::execution_resources::GasAmount(
                                                    1000000,
                                                ),
                                            max_price_per_unit: starknet_api::block::GasPrice(1),
                                        },
                                        l1_data_gas:
                                            starknet_api::transaction::fields::ResourceBounds {
                                                max_amount:
                                                    starknet_api::execution_resources::GasAmount(
                                                        1000000,
                                                    ),
                                                max_price_per_unit: starknet_api::block::GasPrice(
                                                    1,
                                                ),
                                            },
                                        l2_gas: starknet_api::transaction::fields::ResourceBounds {
                                            max_amount:
                                                starknet_api::execution_resources::GasAmount(
                                                    1000000,
                                                ),
                                            max_price_per_unit: starknet_api::block::GasPrice(1),
                                        },
                                    },
                                ),
                            tip: starknet_api::transaction::fields::Tip(0),
                            signature: starknet_api::transaction::fields::TransactionSignature(
                                vec![],
                            ),
                            nonce: starknet_api::core::Nonce(core_types::Felt::ZERO),
                            class_hash: starknet_api::core::ClassHash(*class_hash),
                            contract_address_salt:
                                starknet_api::transaction::fields::ContractAddressSalt(salt),
                            constructor_calldata: starknet_api::transaction::fields::Calldata(
                                Arc::new(calldata_felts.clone()),
                            ),
                            nonce_data_availability_mode:
                                starknet_api::data_availability::DataAvailabilityMode::L2,
                            fee_data_availability_mode:
                                starknet_api::data_availability::DataAvailabilityMode::L2,
                            paymaster_data: starknet_api::transaction::fields::PaymasterData(
                                vec![],
                            ),
                        };

                        let hash = tx
                            .calculate_transaction_hash(
                                &starknet_config.chain_id.into(),
                                &starknet_api::transaction::TransactionVersion::THREE,
                            )
                            .unwrap();

                        let signature = signing_key.sign(&hash).unwrap();

                        tx.signature =
                            starknet_api::transaction::fields::TransactionSignature(vec![
                                signature.r,
                                signature.s,
                            ]);

                        let tx = BroadcastedDeployAccountTransactionV3::from(tx);

                        let z = starknet.add_deploy_account_transaction(
                            BroadcastedDeployAccountTransaction::V3(tx),
                        );

                        Ok(Outcome::Cheat(
                            instruction::CheatcodesReturn::CreateAccount(account_address.into()),
                        ))
                    }
                }
                instruction::CheatInstruction::CreateBlock => {
                    trace!("Environment. Received CreateBlock instruction");

                    let block = starknet.get_latest_block().unwrap();

                    starknet
                        .create_block()
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let events = starknet
                        .get_unlimited_events(
                            Some(core_types::BlockId::Hash(block.block_hash())),
                            None,
                            None,
                            None,
                        )
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    let converted = events.iter().map(|e| EmittedEvent::from(e)).collect();

                    event_broadcaster.send(converted).unwrap_or_default();

                    Ok(Outcome::Cheat(instruction::CheatcodesReturn::CreateBlock))
                }
                instruction::CheatInstruction::L1Message {
                    l1_handler_transaction,
                } => {
                    trace!(
                        "Environment. Received L1Message instruction: {:?}",
                        l1_handler_transaction
                    );

                    let res = starknet
                        .add_l1_handler_transaction(l1_handler_transaction.clone())
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;
                    Ok(Outcome::Cheat(instruction::CheatcodesReturn::L1Message(
                        res,
                    )))
                }
                instruction::CheatInstruction::TopUpBalance {
                    receiver,
                    amount,
                    token,
                } => {
                    trace!(
                        "Environment. Received TopUpBalance instruction: receiver: {:?}, amount: {:?}, token: {:?}",
                        receiver,
                        amount,
                        token
                    );

                    let mut state = starknet.get_state();

                    let receiver = ContractAddress::new(receiver.clone())
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    let token_data = get_token_data(&starknet_config.chain_id, token)
                        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

                    // NOTE: this strategy might not work for all tokens. l2 message strategy should work.
                    // But probably it's better to be implemented on higher level.
                    utils::mint_tokens_in_erc20_contract(
                        &mut state,
                        token_data.l2_token_address,
                        receiver.into(),
                        amount.clone(),
                    )?;

                    starknet.commit_diff()?;

                    Ok(Outcome::Cheat(instruction::CheatcodesReturn::TopUpBalance))
                }
                instruction::CheatInstruction::Impersonate { address } => {
                    trace!(
                        "Environment. Received Impersonate instruction: {:?}",
                        address
                    );

                    starknet
                        .impersonate_account(ContractAddress::new(address.clone()).unwrap())
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    Ok(Outcome::Cheat(instruction::CheatcodesReturn::Impersonate))
                }
                instruction::CheatInstruction::StopImpersonating { address } => {
                    trace!(
                        "Environment. Received StopImpersonating instruction: {:?}",
                        address
                    );

                    starknet.stop_impersonating_account(
                        &ContractAddress::new(address.clone()).unwrap(),
                    );

                    Ok(Outcome::Cheat(
                        instruction::CheatcodesReturn::StopImpersonating,
                    ))
                }
                instruction::CheatInstruction::SetStorageAt {
                    address,
                    key,
                    value,
                } => {
                    trace!(
                        "Environment. Received SetStorageAt instruction: address: {:?}, key: {:?}, value: {:?}",
                        address,
                        key,
                        value
                    );

                    let state = starknet.get_state();

                    let patricia_key = PatriciaKey::try_from(*address).map_err(|e| {
                        ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e))
                    })?;

                    state
                        .state
                        .state
                        .set_storage_at(
                            api_core::ContractAddress(patricia_key),
                            StorageKey::try_from(*key).unwrap(),
                            *value,
                        )
                        .map_err(|e| {
                            ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e))
                        })?;

                    Ok(Outcome::Cheat(instruction::CheatcodesReturn::SetStorageAt))
                }
                instruction::CheatInstruction::GetDeployedContractAddress { tx_hash } => {
                    let receipt = starknet
                        .get_transaction_receipt_by_hash(tx_hash)
                        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                    if let TransactionReceipt::Deploy(deploy_receipt) = receipt {
                        Ok(Outcome::Cheat(
                            instruction::CheatcodesReturn::GetDeployedContractAddress(
                                deploy_receipt.contract_address.into(),
                            ),
                        ))
                    } else {
                        Err(ArbiterCoreError::DevnetError(
                            DevnetError::UnexpectedInternalError {
                                msg: "No deploy events found in tx receipt".to_string(),
                            },
                        ))
                    }
                }
            },
            Instruction::System => todo!(),
        };

        sender
            .send(result)
            .map_err(|_| ArbiterCoreError::SendError(crossbeam_channel::SendError(instruction)))?
    }
    Ok(())
}
