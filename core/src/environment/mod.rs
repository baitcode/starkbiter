//! The [`environment`] module provides abstractions and functionality for
//! handling the Starknet execution environment. This includes managing its
//! state, interfacing with the Starknet, and broadcasting events to subscribers.
//! Other features include the ability to control block rate and gas settings
//! and execute other database modifications from external agents.
//!
//! The key integration for the environment is the Starknet Devnet
//! [`devnet`](https://github.com/0xSpaceShard/starknet-devnet).
//! This is an implementation of the Starkent Sequencer wrapper in Rust that we utilize
//! for processing raw smart contract bytecode.
//!
//! Core structures:
//! - [`Environment`]: Represents the Starknet execution environment, allowing
//!   for its management (e.g., starting, stopping).
//! - [`EnvironmentParameters`]: Parameters necessary for creating or modifying
//!   an [`Environment`].
//! - [`Instruction`]: Enum indicating the type of instruction that is being
//!   sent to the Starknet.

use std::thread::{self, JoinHandle};

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use starknet::providers::Url;
use starknet_core::types as core_types;
use starknet_devnet_core::constants::{self as devnet_constants};

use starknet_devnet_core::error::Error as DevnetError;
use starknet_devnet_core::starknet::starknet_config::ForkConfig;
use starknet_devnet_core::state::{CustomState, CustomStateReader, State};
use starknet_devnet_core::{
    starknet::{starknet_config::StarknetConfig, Starknet},
    state::StateReader,
};
use starknet_devnet_types::chain_id::ChainId;
use starknet_devnet_types::contract_class::ContractClass;

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

use starknet_devnet_types::starknet_api::core as api_core;
use starknet_devnet_types::starknet_api::state::StorageKey;

use tokio::sync::broadcast::channel;

use crate::tokens::get_token_data;

use super::*;

pub mod instruction;
use instruction::{Instruction, NodeInstruction, NodeOutcome, Outcome};

mod utils;

/// Alias for the sender of the channel for transmitting transactions.
pub(crate) type InstructionSender = Sender<(Instruction, OutcomeSender)>;

/// Alias for the receiver of the channel for transmitting transactions.
pub(crate) type InstructionReceiver = Receiver<(Instruction, OutcomeSender)>;

/// Alias for the sender of the channel for transmitting [`RevmResult`] emitted
/// from transactions.
pub(crate) type OutcomeSender = Sender<Result<Outcome, StarkbiterCoreError>>;

/// Alias for the receiver of the channel for transmitting [`RevmResult`]
/// emitted from transactions.
pub(crate) type OutcomeReceiver = Receiver<Result<Outcome, StarkbiterCoreError>>;

/// Represents a sandboxed Starknet environment.
///
/// ## Features
/// * Starknet Devnet and its connections to the "outside world" (agents) via the
///   [`Socket`] provide the [`Environment`] a means to route and execute
///   transactions.
/// * [`EnvironmentParameters`] are used to set the gas limit, contract size
///   limit, and label for the [`Environment`].
#[derive(Debug)]
pub struct Environment {
    /// The label used to define the [`Environment`].
    pub parameters: EnvironmentParameters,

    /// This gives a means of letting the "outside world" connect to the
    /// [`Environment`] so that users (or agents) may send and receive data from
    /// the [`EVM`].
    pub(crate) socket: Socket,

    /// [`JoinHandle`] for the thread in which the Starknet Devnet is running.
    /// Used for assuring that the environment is stopped properly or for
    /// performing any blocking action the end user needs.
    pub(crate) handle: Option<JoinHandle<Result<(), StarkbiterCoreError>>>,
}

/// Parameters to create [`Environment`]s with different settings.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EnvironmentParameters {
    /// The chain ID for the environment, used to identify the network.
    /// IMPORTANT: Be careful when specifying this use anly ascii characters.
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

    /// Sets the chain ID for the [`Environment`].
    pub fn with_chain_id(mut self, chain_id: ChainId) -> Self {
        // TODO: add validation for chain_id so it would contain only ascii characters
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

    fn create(parameters: EnvironmentParameters) -> Self {
        let (instruction_sender, instruction_receiver) = unbounded();
        let (event_broadcaster, _) = channel(512);
        let socket = Socket {
            instruction_sender: Arc::new(instruction_sender),
            instruction_receiver,
            event_broadcaster,
        };

        Self {
            socket,
            parameters,
            handle: None,
        }
    }

    /// This starts the [`Environment`] thread to process any [`Instruction`]s
    /// coming through the [`Socket`].
    fn run(mut self) -> Self {
        // Bring in parameters for the `Environment`.
        let label = self.parameters.label.clone();

        // Pull communication clones to move into a new thread.
        let instruction_receiver: Receiver<(
            Instruction,
            Sender<Result<Outcome, StarkbiterCoreError>>,
        )> = self.socket.instruction_receiver.clone();

        let event_broadcaster = self.socket.event_broadcaster.clone();

        // TODO: fetch block data from remote RPC for better UX.
        let (fork_config, _) = if let Some(url_str) = self.parameters.starknet_fork_url.clone() {
            (
                ForkConfig {
                    url: url_str.parse().ok(),
                    block_number: Some(self.parameters.starknet_fork_block_number.unwrap()),
                    block_hash: Some(self.parameters.starknet_fork_block_hash.unwrap()),
                },
                true,
            )
        } else {
            (ForkConfig::default(), true)
        };

        // Move the Starknet Devnet and its socket to a new thread and retrieve this handle
        let handle = thread::spawn(move || {
            let result = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let starknet_config = &StarknetConfig {
                        fork_config,
                        chain_id: self.parameters.chain_id.unwrap_or(ChainId::Testnet),
                        ..StarknetConfig::default()
                    };

                    // TODO: support forking
                    // TODO: Simulated block production
                    // TODO: every instruction whould encapsulate handling logic, that would allow to simplify the code and split environment mod to smaller chunks.
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

    /// Stops the execution of the environment
    pub fn stop(mut self) -> Result<(), StarkbiterCoreError> {
        let (outcome_sender, outcome_receiver) = bounded(1);

        let to_send = (
            Instruction::System(instruction::SystemInstruction::Stop),
            outcome_sender,
        );

        self.socket.instruction_sender.send(to_send).map_err(|_| {
            StarkbiterCoreError::InternalError("Failed to send stop instruction".to_string())
        })?;

        let _ = outcome_receiver.recv()?;

        if let Some(label) = &self.parameters.label {
            warn!("Stopped environment with label: {}", label);
        } else {
            warn!("Stopped environment with no label.");
        }

        drop(self.socket.instruction_sender);

        self.handle
            .take()
            .unwrap()
            .join()
            .map_err(|_| StarkbiterCoreError::JoinError)??;

        Ok(())
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
    _label: String,
    instruction_receiver: Receiver<(Instruction, Sender<Result<Outcome, StarkbiterCoreError>>)>,
    event_broadcaster: BroadcastSender<Vec<core_types::EmittedEvent>>,
) -> Result<(), StarkbiterCoreError> {
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
    let mut stop = false;

    while let Ok((instruction, sender)) = instruction_receiver.recv() {
        if stop {
            break;
        }

        match instruction {
            Instruction::Node(ref basic_instruction) => match basic_instruction {
                NodeInstruction::GetSpecVersion => {
                    trace!("Environment. Received GetSpecVersion instruction");
                    let outcome = Outcome::Node(NodeOutcome::SpecVersion("unknown".to_string()));
                    // TODO: handle send errors. Probably stop the environment.
                    if let Err(e) = sender.send(Ok(outcome)) {
                        error!("Failed to send GetSpecVersion outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetBlockWithTxHashes { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithTxHashes instruction: {:?}",
                        block_id
                    );
                    let outcome = starknet
                        .get_block_with_transactions(block_id)
                        .map_err(|err| StarkbiterCoreError::DevnetError(err))
                        .map(|block_result| {
                            let outcome: core_types::MaybePendingBlockWithTxHashes =
                                match block_result {
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
                            Outcome::Node(NodeOutcome::GetBlockWithTxHashes(outcome))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetBlockWithTxHashes outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetBlockWithTxs { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithTxs instruction: {:?}",
                        block_id
                    );
                    let outcome = starknet
                        .get_block_with_transactions(&block_id)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|block_result| {
                            let outcome: core_types::MaybePendingBlockWithTxs = match block_result {
                                BlockResult::PendingBlock(block) => {
                                    core_types::MaybePendingBlockWithTxs::PendingBlock(
                                        core_types::PendingBlockWithTxs::from(block),
                                    )
                                }
                                BlockResult::Block(block) => {
                                    core_types::MaybePendingBlockWithTxs::Block(
                                        core_types::BlockWithTxs::from(block),
                                    )
                                }
                            };
                            Outcome::Node(NodeOutcome::GetBlockWithTxs(outcome))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetBlockWithTxs outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetBlockWithReceipts { block_id } => {
                    trace!(
                        "Environment. Received GetBlockWithReceipts instruction: {:?}",
                        block_id
                    );
                    let outcome = starknet
                        .get_block_with_receipts(&block_id)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|block_result| {
                            let outcome: core_types::MaybePendingBlockWithReceipts =
                                match block_result {
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
                            Outcome::Node(NodeOutcome::GetBlockWithReceipts(outcome))
                        });
                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetBlockWithReceipts outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetStateUpdate { block_id } => {
                    trace!(
                        "Environment. Received GetStateUpdate instruction: {:?}",
                        block_id
                    );

                    let outcome = starknet
                        .block_state_update(block_id)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|res| Outcome::Node(NodeOutcome::GetStateUpdate(res.into())));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetStateUpdate outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetStorageAt {
                    contract_address,
                    key,
                    // TODO: hmmmm, something is off
                    block_id,
                } => {
                    let operation = "GetStorageAt";

                    trace!(
                        "Environment. Received {} instruction: {:?} {:?} {:?}",
                        operation,
                        contract_address,
                        key,
                        block_id
                    );
                    let state = starknet.get_state();

                    let maybe_contract_address =
                        api_core::ContractAddress::try_from(*contract_address);

                    if let Err(err) = maybe_contract_address {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(err),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send {} outcome: {:?}", operation, e);
                            stop = true;
                        }

                        continue;
                    }

                    let maybe_storage_key = StorageKey::try_from(*key);

                    if let Err(err) = maybe_storage_key {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(err),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send {} outcome: {:?}", operation, e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = state
                        .get_storage_at(maybe_contract_address.unwrap(), maybe_storage_key.unwrap())
                        .map_err(|e| {
                            StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e))
                        })
                        .map(|value| Outcome::Node(NodeOutcome::GetStorageAt(value)));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetStorageAt outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetMessagesStatus { transaction_hash } => {
                    trace!(
                        "Environment. Received GetMessagesStatus instruction: {:?}",
                        transaction_hash
                    );

                    let outcome = starknet
                        .get_messages_status(*transaction_hash)
                        .map(|status| {
                            Outcome::Node(NodeOutcome::GetMessagesStatus(
                                status.iter().map(Into::into).collect(),
                            ))
                        })
                        .unwrap_or(Outcome::Node(NodeOutcome::GetMessagesStatus(vec![])));

                    if let Err(e) = sender.send(Ok(outcome)) {
                        error!("Failed to send GetMessagesStatus outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetTransactionStatus { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionStatus instruction: {:?}",
                        transaction_hash
                    );

                    let hash = core_types::Felt::try_from(transaction_hash);

                    if let Err(e) = hash {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send GetTransactionStatus outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = starknet
                        .get_transaction_execution_and_finality_status(hash.unwrap())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|status| {
                            Outcome::Node(NodeOutcome::GetTransactionStatus(status.into()))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetTransactionStatus outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetTransactionByHash { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionByHash instruction: {:?}",
                        transaction_hash
                    );

                    let hash = core_types::Felt::try_from(transaction_hash);

                    if let Err(e) = hash {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send GetTransactionByHash outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = starknet
                        .get_transaction_by_hash(hash.unwrap())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|transaction| {
                            Outcome::Node(NodeOutcome::GetTransactionByHash(
                                core_types::Transaction::from(transaction.clone()),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetTransactionByHash outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetTransactionByBlockIdAndIndex { block_id, index } => {
                    trace!(
                        "Environment. Received GetTransactionByBlockIdAndIndex instruction: {:?} {:?}",
                        block_id,
                        index
                    );

                    let outcome = starknet
                        .get_transaction_by_block_id_and_index(&block_id, *index)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|tx| {
                            Outcome::Node(NodeOutcome::GetTransactionByBlockIdAndIndex(
                                core_types::Transaction::from(tx.clone()),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetTransactionByHash outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetTransactionReceipt { transaction_hash } => {
                    trace!(
                        "Environment. Received GetTransactionReceipt instruction: {:?}",
                        transaction_hash
                    );

                    let hash = core_types::Felt::try_from(transaction_hash);

                    if let Err(e) = hash {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send GetTransactionReceipt outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = starknet
                        .get_transaction_receipt_by_hash(&hash.unwrap())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|receipt| {
                            Outcome::Node(NodeOutcome::GetTransactionReceipt(
                                core_types::TransactionReceiptWithBlockInfo::from(receipt),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetTransactionReceipt outcome: {:?}", e);
                        stop = true;
                    }
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

                    let outcome = starknet
                        .get_class(&block_id, *class_hash)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|klass| {
                            Outcome::Node(NodeOutcome::GetClass(
                                klass
                                    .try_into()
                                    .expect("Could not convert between ContractClasses"),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetClass outcome: {:?}", e);
                        stop = true;
                    }
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

                    let contract_address = ContractAddress::new(*contract_address);

                    if let Err(e) = contract_address {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send GetClass outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = starknet
                        .get_class_hash_at(&block_id, contract_address.unwrap())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|class_hash| Outcome::Node(NodeOutcome::GetClassHashAt(class_hash)));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetClass outcome: {:?}", e);
                        stop = true;
                    }
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

                    let outcome = starknet
                        .get_class_at(&block_id, contract_address)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|contract_class| {
                            Outcome::Node(NodeOutcome::GetClassAt(
                                contract_class
                                    .try_into()
                                    .expect("Convert between contract classes"),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetClass outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::GetBlockTransactionCount { block_id } => {
                    trace!(
                        "Environment. Received GetBlockTransactionCount instruction: {:?}",
                        block_id
                    );

                    let outcome = starknet
                        .get_block_txs_count(&block_id)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|count| Outcome::Node(NodeOutcome::GetBlockTransactionCount(count)));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetBlockTransactionCount outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::BlockNumber => {
                    trace!("Environment. Received BlockNumber instruction");

                    let outcome = starknet
                        .get_latest_block()
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|block| {
                            Outcome::Node(NodeOutcome::BlockNumber(block.block_number().0))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send BlockNumber outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::BlockHashAndNumber => {
                    trace!("Environment. Received BlockHashAndNumber instruction");

                    let outcome = starknet
                        .get_latest_block()
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|block| {
                            Outcome::Node(NodeOutcome::BlockHashAndNumber(
                                core_types::BlockHashAndNumber {
                                    block_hash: block.block_hash(),
                                    block_number: block.block_number().0,
                                },
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send BlockHashAndNumber outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::ChainId => {
                    trace!("Environment. Received ChainId instruction");

                    let chain_id = starknet_config.chain_id;
                    let outcome = Ok(Outcome::Node(NodeOutcome::ChainId(chain_id.into())));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send ChainId outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::Syncing => {
                    let outcome = Ok(Outcome::Node(NodeOutcome::Syncing(
                        core_types::SyncStatusType::NotSyncing,
                    )));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send Syncing outcome: {:?}", e);
                        stop = true;
                    }
                }
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
                        s.parse::<u64>().unwrap_or(0)
                    } else {
                        0
                    };

                    let chunk_size: usize = if let Some(size) = chunk_size {
                        *size as usize
                    } else {
                        50_usize // TODO: Default chunk size. Move to config
                    };

                    let outcome = starknet
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
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|(events, _)| {
                            let continuation_token = if events.len() < chunk_size {
                                Option::None
                            } else {
                                Option::Some(format!("{}", skip + events.len() as u64))
                            };

                            let page = core_types::EventsPage {
                                events: events.iter().map(|e| e.into()).collect(),
                                continuation_token,
                            };

                            Outcome::Node(NodeOutcome::GetEvents(page))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetEvents outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::Call { request, block_id } => {
                    trace!(
                        "Environment. Received Call instruction: {:?} {:?}",
                        request,
                        block_id
                    );

                    let outcome = starknet
                        .call(
                            block_id,
                            request.contract_address,
                            request.entry_point_selector,
                            request.calldata.clone(),
                        )
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|res| Outcome::Node(NodeOutcome::Call(res)));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send Call outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::AddInvokeTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddInvokeTransaction instruction: {:?}",
                        transaction
                    );

                    let converted_transaction =
                        BroadcastedInvokeTransaction::V3(transaction.clone().into());

                    let outcome = starknet
                        .add_invoke_transaction(converted_transaction)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|tx_hash| {
                            Outcome::Node(NodeOutcome::AddInvokeTransaction(
                                core_types::InvokeTransactionResult {
                                    transaction_hash: tx_hash,
                                },
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send AddInvokeTransaction outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::AddDeclareTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddDeclareTransaction instruction: {:?}",
                        transaction
                    );

                    let outcome = starknet
                        .add_declare_transaction(BroadcastedDeclareTransaction::V3(Box::new(
                            transaction.clone().into(),
                        )))
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|(tx_hash, class_hash)| {
                            Outcome::Node(NodeOutcome::AddDeclareTransaction(
                                core_types::DeclareTransactionResult {
                                    transaction_hash: tx_hash,
                                    class_hash,
                                },
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send AddDeclareTransaction outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::AddDeployAccountTransaction { transaction } => {
                    trace!(
                        "Environment. Received AddDeployAccountTransaction instruction: {:?}",
                        transaction
                    );

                    let outcome = starknet
                        .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(
                            transaction.clone().into(),
                        ))
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|(tx_hash, contract_address)| {
                            Outcome::Node(NodeOutcome::AddDeployAccountTransaction(
                                core_types::DeployAccountTransactionResult {
                                    transaction_hash: tx_hash,
                                    contract_address: core_types::Felt::from(contract_address),
                                },
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!(
                            "Failed to send AddDeployAccountTransaction outcome: {:?}",
                            e
                        );
                        stop = true;
                    }
                }
                NodeInstruction::TraceTransaction { transaction_hash } => {
                    trace!(
                        "Environment. Received TraceTransaction instruction: {:?}",
                        transaction_hash
                    );

                    let outcome = starknet
                        .get_transaction_trace_by_hash(*transaction_hash)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|trace| Outcome::Node(NodeOutcome::TraceTransaction(trace.into())));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send TraceTransaction outcome: {:?}", e);
                        stop = true;
                    }
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

                    let outcome = starknet
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
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|res| {
                            Outcome::Node(NodeOutcome::SimulateTransactions(
                                res.iter().cloned().map(Into::into).collect(),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send SimulateTransactions outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::TraceBlockTransactions { block_id } => {
                    trace!(
                        "Environment. Received TraceBlockTransactions instruction: {:?}",
                        block_id
                    );

                    let outcome = starknet
                        .get_transaction_traces_from_block(&block_id)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|res| {
                            Outcome::Node(NodeOutcome::TraceBlockTransactions(
                                res.iter().cloned().map(Into::into).collect(),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send TraceBlockTransactions outcome: {:?}", e);
                        stop = true;
                    }
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

                    let fees_result =
                        starknet.estimate_fee(&block_id, txs, &simulation_flags.as_slice());

                    let outcome = match fees_result {
                        Err(e) => Err(StarkbiterCoreError::DevnetError(e)),
                        Ok(fees) => Ok(Outcome::Node(NodeOutcome::EstimateFee(
                            fees.iter().cloned().map(Into::into).collect(),
                        ))),
                    };

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send TraceBlockTransactions outcome: {:?}", e);
                        stop = true;
                    }
                }
                NodeInstruction::EstimateMessageFee { message, block_id } => {
                    trace!(
                        "Environment. Received EstimateMessageFee instruction: {:?} {:?}",
                        message,
                        block_id
                    );

                    let outcome = starknet
                        .estimate_message_fee(&block_id, message.clone())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|fee| Outcome::Node(NodeOutcome::EstimateMessageFee(fee.into())));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send EstimateMessageFee outcome: {:?}", e);
                        stop = true;
                    }
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

                    let outcome = starknet
                        .contract_nonce_at_block(&block_id, contract_address)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|nonce| Outcome::Node(NodeOutcome::GetNonce(nonce.into())));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetNonce outcome: {:?}", e);
                        stop = true;
                    }
                }
            },
            Instruction::Cheat(ref cheat_instruction) => match cheat_instruction {
                instruction::CheatInstruction::SetNextBlockGas { gas_modification } => {
                    trace!("Environment. Received SetNextBlockGas instruction");

                    let outcome = starknet
                        .set_next_block_gas(gas_modification.clone())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|modification| {
                            Outcome::Cheat(instruction::CheatcodesReturn::SetNextBlockGas(
                                modification,
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send SetNextBlockGas outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::DeclareContract { sierra_json } => {
                    trace!("Environment. Received DeclareContract instruction: ",);

                    let contract_class = ContractClass::cairo_1_from_sierra_json_str(&sierra_json);

                    if let Err(err) = contract_class {
                        let outcome = Err(StarkbiterCoreError::InternalError(err.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send DeclareContract outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let sierra_contract_class = ContractClass::Cairo1(contract_class.unwrap());
                    let class_hash_result = sierra_contract_class.generate_hash();

                    if let Err(err) = class_hash_result {
                        let outcome = Err(StarkbiterCoreError::InternalError(err.to_string()));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send DeclareContract outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let class_hash = class_hash_result.unwrap();

                    trace!("Class hash generated: {:?}", class_hash);
                    let state = starknet.get_state();

                    if !state.is_contract_declared(class_hash) {
                        trace!("Contract class predeclared");

                        let declare_result = state
                            .predeclare_contract_class(class_hash, sierra_contract_class.clone());

                        if let Err(e) = declare_result {
                            let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));

                            if let Err(e) = sender.send(outcome) {
                                error!("Failed to send DeclareContract outcome: {:?}", e);
                                stop = true;
                            }

                            continue;
                        }
                    }

                    let commit_result = starknet
                        .commit_diff()
                        .map_err(|e| StarkbiterCoreError::DevnetError(e.into()));

                    if let Err(e) = commit_result {
                        if let Err(e) = sender.send(Err(e)) {
                            error!("Failed to send DeclareContract outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let class_hash = api_core::ClassHash(class_hash);

                    let outcome = Ok(Outcome::Cheat(
                        instruction::CheatcodesReturn::DeclareContract(class_hash.0),
                    ));
                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send DeclareContract outcome: {:?}", e);
                        stop = true;
                    }
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
                    );

                    if let Err(e) = account_address {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(e),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send CreateAccount outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let is_account_contract_declared = {
                        let state = starknet.get_state();
                        state.is_contract_declared(*class_hash)
                    };

                    {
                        let mut state = starknet.get_state();
                        trace!("Minting tokens...");
                        if let Err(e) = utils::mint_tokens_in_erc20_contract(
                            &mut state,
                            devnet_constants::STRK_ERC20_CONTRACT_ADDRESS,
                            account_address.unwrap().into(),
                            prefunded_balance.clone(),
                        ) {
                            error!("Error minting tokens for new account: {:?}", e);
                        }
                        if let Err(e) = starknet.commit_diff() {
                            error!("Error commiting diff to devnet: {:?}", e);
                        }
                    }

                    if !is_account_contract_declared {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::ContractClassLoadError("Not declared".to_string()),
                        ));
                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send CreateAccount outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let mut tx = starknet_api::transaction::DeployAccountTransactionV3 {
                        resource_bounds:
                            starknet_api::transaction::fields::ValidResourceBounds::AllResources(
                                starknet_api::transaction::fields::AllResourceBounds {
                                    l1_gas: starknet_api::transaction::fields::ResourceBounds {
                                        max_amount: starknet_api::execution_resources::GasAmount(
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
                                            max_price_per_unit: starknet_api::block::GasPrice(1),
                                        },
                                    l2_gas: starknet_api::transaction::fields::ResourceBounds {
                                        max_amount: starknet_api::execution_resources::GasAmount(
                                            1000000,
                                        ),
                                        max_price_per_unit: starknet_api::block::GasPrice(1),
                                    },
                                },
                            ),
                        tip: starknet_api::transaction::fields::Tip(0),
                        signature: starknet_api::transaction::fields::TransactionSignature(vec![]),
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
                        paymaster_data: starknet_api::transaction::fields::PaymasterData(vec![]),
                    };

                    let hash = tx.calculate_transaction_hash(
                        &starknet_config.chain_id.into(),
                        &starknet_api::transaction::TransactionVersion::THREE,
                    );

                    if let Err(e) = hash {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(e),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send CreateAccount outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let signature = signing_key.sign(&hash.unwrap()).unwrap();

                    tx.signature = starknet_api::transaction::fields::TransactionSignature(vec![
                        signature.r,
                        signature.s,
                    ]);

                    let tx = BroadcastedDeployAccountTransactionV3::from(tx);

                    let outcome = starknet
                        .add_deploy_account_transaction(BroadcastedDeployAccountTransaction::V3(tx))
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|(_, address)| {
                            Outcome::Cheat(instruction::CheatcodesReturn::CreateAccount(
                                address.into(),
                            ))
                        });

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send CreateAccount outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::CreateBlock => {
                    trace!("Environment. Received CreateBlock instruction");

                    let block = starknet.get_latest_block().unwrap();

                    let create_block_result = starknet
                        .create_block()
                        .map_err(|e| StarkbiterCoreError::DevnetError(e));

                    if let Err(e) = create_block_result {
                        if let Err(e) = sender.send(Err(e)) {
                            error!("Failed to send CreateBlock outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let events = starknet
                        .get_unlimited_events(
                            Some(core_types::BlockId::Hash(block.block_hash())),
                            None,
                            None,
                            None,
                        )
                        .map_err(|e| StarkbiterCoreError::DevnetError(e));

                    if let Err(e) = events {
                        if let Err(e) = sender.send(Err(e)) {
                            error!("Failed to send CreateBlock outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let converted = events
                        .unwrap()
                        .iter()
                        .map(|e| core_types::EmittedEvent::from(e))
                        .collect();

                    event_broadcaster.send(converted).unwrap_or_default();

                    let outcome = Ok(Outcome::Cheat(instruction::CheatcodesReturn::CreateBlock));
                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send CreateBlock outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::L1Message {
                    l1_handler_transaction,
                } => {
                    trace!(
                        "Environment. Received L1Message instruction: {:?}",
                        l1_handler_transaction
                    );

                    let outcome = starknet
                        .add_l1_handler_transaction(l1_handler_transaction.clone())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|res| Outcome::Cheat(instruction::CheatcodesReturn::L1Message(res)));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send L1Message outcome: {:?}", e);
                        stop = true;
                    }
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

                    let receiver = ContractAddress::new(receiver.clone());
                    if let Err(e) = receiver {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));
                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send TopUpBalance outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let token_data = get_token_data(&starknet_config.chain_id, token);
                    if let Err(e) = token_data {
                        let outcome = Err(StarkbiterCoreError::InternalError(e.to_string()));
                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send TopUpBalance outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    // NOTE: this strategy might not work for all tokens. l2 message strategy should work.
                    // But probably it's better to be implemented on higher level.
                    let mint_result = utils::mint_tokens_in_erc20_contract(
                        &mut state,
                        token_data.unwrap().l2_token_address,
                        receiver.unwrap().into(),
                        amount.clone(),
                    );

                    if let Err(e) = mint_result {
                        if let Err(e) = sender.send(Err(e)) {
                            error!("Failed to send TopUpBalance outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let res = starknet.commit_diff();
                    if let Err(e) = res {
                        let outcome = Err(StarkbiterCoreError::DevnetError(e));
                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send TopUpBalance outcome: {:?}", e);
                            stop = true;
                        }
                        continue;
                    }

                    let outcome = Ok(Outcome::Cheat(instruction::CheatcodesReturn::TopUpBalance));
                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send TopUpBalance outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::Impersonate { address } => {
                    trace!(
                        "Environment. Received Impersonate instruction: {:?}",
                        address
                    );

                    let outcome = starknet
                        .impersonate_account(ContractAddress::new(address.clone()).unwrap())
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))
                        .map(|_| Outcome::Cheat(instruction::CheatcodesReturn::Impersonate));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send Impersonate outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::StopImpersonating { address } => {
                    trace!(
                        "Environment. Received StopImpersonating instruction: {:?}",
                        address
                    );

                    starknet.stop_impersonating_account(
                        &ContractAddress::new(address.clone()).unwrap(),
                    );

                    let outcome = Ok(Outcome::Cheat(
                        instruction::CheatcodesReturn::StopImpersonating,
                    ));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send Impersonate outcome: {:?}", e);
                        stop = true;
                    }
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

                    let maybe_address = api_core::ContractAddress::try_from(*address);

                    if let Err(err) = maybe_address {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(err),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send SetStorageAt outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let maybe_storage_key = StorageKey::try_from(*key);

                    if let Err(err) = maybe_storage_key {
                        let outcome = Err(StarkbiterCoreError::DevnetError(
                            DevnetError::StarknetApiError(err),
                        ));

                        if let Err(e) = sender.send(outcome) {
                            error!("Failed to send SetStorageAt outcome: {:?}", e);
                            stop = true;
                        }

                        continue;
                    }

                    let outcome = state
                        .state
                        .set_storage_at(maybe_address.unwrap(), maybe_storage_key.unwrap(), *value)
                        .map_err(|e| {
                            StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e))
                        })
                        .map(|_| Outcome::Cheat(instruction::CheatcodesReturn::SetStorageAt));

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send SetStorageAt outcome: {:?}", e);
                        stop = true;
                    }
                }
                instruction::CheatInstruction::GetDeployedContractAddress { tx_hash } => {
                    let receipt = starknet
                        .get_transaction_receipt_by_hash(tx_hash)
                        .map_err(|e| StarkbiterCoreError::DevnetError(e))?;

                    let outcome = if let TransactionReceipt::Deploy(deploy_receipt) = receipt {
                        Ok(Outcome::Cheat(
                            instruction::CheatcodesReturn::GetDeployedContractAddress(
                                deploy_receipt.contract_address.into(),
                            ),
                        ))
                    } else {
                        Err(StarkbiterCoreError::DevnetError(
                            DevnetError::UnexpectedInternalError {
                                msg: "No deploy events found in tx receipt".to_string(),
                            },
                        ))
                    };

                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send GetDeployedContractAddress outcome: {:?}", e);
                        stop = true;
                    }
                }
            },
            Instruction::System(ref system_instruction) => match system_instruction {
                instruction::SystemInstruction::Stop => {
                    trace!("Environment. Received Stop instruction");
                    stop = true;

                    let outcome = Ok(Outcome::System(instruction::SystemInstructionOutcome::Stop));
                    if let Err(e) = sender.send(outcome) {
                        error!("Failed to send Stop outcome: {:?}", e);
                        stop = true;
                    }
                }
            },
        };
    }
    Ok(())
}
