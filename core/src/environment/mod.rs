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

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};

use polars::prelude::ArrayCollectIterExt;
use starknet::contract;
use starknet_core::types::{self as core_types};
use starknet_devnet_core::constants::{
    self, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
};

use starknet_devnet_core::error::Error as DevnetError;
use starknet_devnet_core::{
    starknet::{starknet_config::StarknetConfig, Starknet},
    state::{StarknetState, StateReader},
};
use starknet_devnet_types::error::Error;
use starknet_devnet_types::rpc::transactions::broadcasted_invoke_transaction_v3::BroadcastedInvokeTransactionV3;
use starknet_devnet_types::rpc::transactions::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction,
    BroadcastedInvokeTransaction, BroadcastedTransaction, SimulationFlag, TransactionTrace,
};
use starknet_devnet_types::{contract_address::ContractAddress, num_bigint::BigUint};

use starknet_devnet_types::rpc::block::{self, BlockResult};

use starknet_devnet_types::starknet_api::state::StorageKey;
use starknet_devnet_types::starknet_api::{core as api_core, transaction_hash};
use tokio::sync::broadcast::channel;

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
    /// The label used to define the [`Environment`].
    pub label: Option<String>,

    /// The gas limit for the blocks in the [`Environment`].
    pub gas_limit: Option<BigUint>,

    /// The contract size limit for the [`Environment`].
    pub contract_size_limit: Option<usize>,

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
    // db: ArbiterDB,
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

    /// Sets the label for the [`Environment`].
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.parameters.label = Some(label.into());
        self
    }

    /// Sets the gas limit for the [`Environment`].
    pub fn with_gas_limit(mut self, gas_limit: BigUint) -> Self {
        self.parameters.gas_limit = Some(gas_limit);
        self
    }

    /// Sets the contract size limit for the [`Environment`].
    pub fn with_contract_size_limit(mut self, contract_size_limit: usize) -> Self {
        self.parameters.contract_size_limit = Some(contract_size_limit);
        self
    }

    /// Sets the state for the [`Environment`]. This can come from a saved state
    /// of a simulation or a [`database::fork::Fork`].
    // pub fn with_state(mut self, state: impl Into<CacheDB<EmptyDB>>) -> Self {
    //     self.db.state = Arc::new(RwLock::new(state.into()));
    //     self
    // }

    /// Sets the logs for the [`Environment`]. This can come from a saved state
    /// of a simulation and can be useful for doing analysis.
    // pub fn with_logs(
    //     mut self,
    //     logs: impl Into<std::collections::HashMap<U256, Vec<eLog>>>,
    // ) -> Self {
    //     self.db.logs = Arc::new(RwLock::new(logs.into()));
    //     self
    // }

    /// Sets the entire database for the [`Environment`] including both the
    /// state and logs. This can come from the saved state of a simulation and
    /// can be useful for doing analysis.
    pub fn with_arbiter_db(
        mut self,
        // db: ArbiterDB
    ) -> Self {
        // self.db = db;
        self
    }

    /// Enables inner contract logs to be printed to the console as `trace`
    /// level logs prepended with "Console logs: ".
    pub fn with_console_logs(mut self) -> Self {
        self.parameters.console_logs = true;
        self
    }

    /// Turns on gas payments for transactions so that the [`EVM`] will
    /// automatically pay for gas and revert if balance is not met by sender.
    pub fn with_pay_gas(mut self) -> Self {
        self.parameters.pay_gas = true;
        self
    }
}

impl Environment {
    /// Creates a new [`EnvironmentBuilder`] with default parameters that can be
    /// used to build an [`Environment`].
    pub fn builder() -> EnvironmentBuilder {
        EnvironmentBuilder {
            parameters: EnvironmentParameters::default(),
            // db: ArbiterDB::default(),
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

        // let inspector = if parameters.console_logs || parameters.pay_gas {
        //     Some(ArbiterInspector::new(
        //         parameters.console_logs,
        //         parameters.pay_gas,
        //     ))
        // } else {
        //     Some(ArbiterInspector::new(false, false))
        // };

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
        let instruction_receiver = self.socket.instruction_receiver.clone();
        let event_broadcaster = self.socket.event_broadcaster.clone();

        // Move the EVM and its socket to a new thread and retrieve this handle
        let handle = thread::spawn(move || {
            // TODO: support forking
            // TODO: Simulated block production

            // let mut env = Env::default();
            // env.cfg.limit_contract_code_size = self.parameters.contract_size_limit;
            // env.block.gas_limit = self.parameters.gas_limit.unwrap_or(eU256::MAX);

            let mut starknet_config = &StarknetConfig::default();
            // Fork configuration

            let mut starknet = Starknet::new(starknet_config).unwrap();

            // Initialize counters that are returned on some receipts.
            // let mut transaction_index = 0_u64;
            // let mut last_executed_gas_price: u128 = 0;
            // let mut cumulative_gas_per_block = BigUint::from(0_u128);

            // Loop over the instructions sent through the socket.
            while let Ok((instruction, sender)) = instruction_receiver.recv() {
                trace!(
                    "Instruction {:?} received by environment labeled: {:?}",
                    instruction,
                    label
                );

                let result: Result<Outcome, ArbiterCoreError> = match instruction {
                    Instruction::Node(ref basic_instruction) => match basic_instruction {
                        NodeInstruction::GetSpecVersion => {
                            let outcome =
                                Outcome::Node(NodeOutcome::SpecVersion("unknown".to_string()));
                            Ok(outcome)
                        }

                        NodeInstruction::GetBlockWithTxHashes { block_id } => {
                            let block_result = starknet
                                .get_block_with_transactions(block_id)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

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

                            Ok(Outcome::Node(NodeOutcome::GetBlockWithTxHashes(outcome)))
                        }

                        NodeInstruction::GetBlockWithTxs { block_id } => {
                            let block_result = starknet
                                .get_block_with_transactions(&block_id)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

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

                            Ok(Outcome::Node(NodeOutcome::GetBlockWithTxs(outcome)))
                        }

                        NodeInstruction::GetBlockWithReceipts { block_id } => {
                            let block_result = starknet
                                .get_block_with_receipts(&block_id)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

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

                            Ok(Outcome::Node(NodeOutcome::GetBlockWithReceipts(outcome)))
                        }

                        NodeInstruction::GetStateUpdate { block_id } => {
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
                            let state = starknet.get_state();

                            let contract_address = api_core::ContractAddress::try_from(
                                *contract_address,
                            )
                            .map_err(|e| {
                                ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e))
                            })?;

                            let storage_key = StorageKey::try_from(*key).map_err(|e| {
                                ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e))
                            })?;

                            let outcome = state
                                .get_storage_at(contract_address, storage_key)
                                .map_err(|e| {
                                    ArbiterCoreError::DevnetError(
                                        DevnetError::BlockifierStateError(e),
                                    )
                                })?;

                            Ok(Outcome::Node(NodeOutcome::GetStorageAt(outcome)))
                        }

                        NodeInstruction::GetMessagesStatus { transaction_hash } => {
                            let outcome = starknet
                                .get_messages_status(*transaction_hash)
                                .unwrap_or(Vec::new());
                            Ok(Outcome::Node(NodeOutcome::GetMessagesStatus(
                                outcome.iter().map(Into::into).collect(),
                            )))
                        }

                        NodeInstruction::GetTransactionStatus { transaction_hash } => {
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
                            let transaction = starknet
                                .get_transaction_by_block_id_and_index(&block_id, *index)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetTransactionByBlockIdAndIndex(
                                core_types::Transaction::from(transaction.clone()),
                            )))
                        }
                        NodeInstruction::GetTransactionReceipt { transaction_hash } => {
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
                            let klass = starknet
                                .get_class(&block_id, *class_hash)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetClass(
                                klass.try_into().map_err(|e: Error| {
                                    ArbiterCoreError::InternalError(e.to_string())
                                })?,
                            )))
                        }
                        NodeInstruction::GetClassHashAt {
                            block_id,
                            contract_address,
                        } => {
                            let contract_address = ContractAddress::new(*contract_address)
                                .expect("Should always work");

                            let class_hash = starknet
                                .get_class_hash_at(&block_id, contract_address)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetClassHashAt(class_hash)))
                        }
                        NodeInstruction::GetClassAt {
                            block_id,
                            contract_address,
                        } => {
                            let contract_address = ContractAddress::new(*contract_address)
                                .expect("Should always work");

                            let contract_class = starknet
                                .get_class_at(&block_id, contract_address)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetClassAt(
                                contract_class.try_into().map_err(|e: Error| {
                                    ArbiterCoreError::InternalError(e.to_string())
                                })?,
                            )))
                        }
                        NodeInstruction::GetBlockTransactionCount { block_id } => {
                            let count = starknet
                                .get_block_txs_count(&block_id)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetBlockTransactionCount(count)))
                        }
                        NodeInstruction::BlockNumber => {
                            let block = starknet
                                .get_latest_block()
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::BlockNumber(
                                block.block_number().0,
                            )))
                        }
                        NodeInstruction::BlockHashAndNumber => {
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
                            let chain_id = starknet.config.chain_id;
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
                                        Some(address) => Some(
                                            ContractAddress::new(address)
                                                .expect("Should always work"),
                                        ),
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
                            let res = starknet
                                .call(
                                    block_id,
                                    request.contract_address,
                                    request.entry_point_selector,
                                    request.calldata.clone(),
                                )
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::Call(res)))
                        }

                        NodeInstruction::AddInvokeTransaction { transaction } => {
                            let tx_hash = starknet
                                .add_invoke_transaction(BroadcastedInvokeTransaction::V3(
                                    transaction.clone().into(),
                                ))
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::AddInvokeTransaction(
                                core_types::InvokeTransactionResult {
                                    transaction_hash: tx_hash,
                                },
                            )))
                        }
                        NodeInstruction::AddDeclareTransaction { transaction } => {
                            let (tx_hash, class_hash) = starknet
                                .add_declare_transaction(BroadcastedDeclareTransaction::V3(
                                    Box::new(transaction.clone().into()),
                                ))
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::AddDeclareTransaction(
                                core_types::DeclareTransactionResult {
                                    transaction_hash: tx_hash,
                                    class_hash,
                                },
                            )))
                        }
                        NodeInstruction::AddDeployAccountTransaction { transaction } => {
                            let (tx_hash, contract_address) = starknet
                                .add_deploy_account_transaction(
                                    BroadcastedDeployAccountTransaction::V3(
                                        transaction.clone().into(),
                                    ),
                                )
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::AddDeployAccountTransaction(
                                core_types::DeployAccountTransactionResult {
                                    transaction_hash: tx_hash,
                                    contract_address: core_types::Felt::from(contract_address),
                                },
                            )))
                        }
                        NodeInstruction::TraceTransaction { transaction_hash } => {
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
                            let txs = &[BroadcastedTransaction::from(request.clone())];

                            // NOTE: good example of From trait superiority
                            let simulation_flags = &[SimulationFlag::from(*simulate_flags)];

                            let fees = starknet
                                .estimate_fee(&block_id, txs, simulation_flags)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::EstimateFee(
                                fees.iter().cloned().map(Into::into).collect(),
                            )))
                        }
                        NodeInstruction::EstimateMessageFee { message, block_id } => {
                            let fee = starknet
                                .estimate_message_fee(&block_id, message.clone())
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::EstimateMessageFee(fee.into())))
                        }
                        NodeInstruction::GetNonce {
                            block_id,
                            contract_address,
                        } => {
                            let contract_address = ContractAddress::new(*contract_address)
                                .expect("Should always work.");

                            let nonce = starknet
                                .contract_nonce_at_block(&block_id, contract_address)
                                .map_err(|e| ArbiterCoreError::DevnetError(e))?;

                            Ok(Outcome::Node(NodeOutcome::GetNonce(nonce.into())))
                        }
                    },
                    Instruction::Cheat => todo!(),
                    Instruction::System => todo!(),
                };

                sender.send(result).map_err(|_| {
                    ArbiterCoreError::SendError(crossbeam_channel::SendError(instruction))
                })?
            }
            Ok(())
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
    pub(crate) event_broadcaster: BroadcastSender<Broadcast>,
}

/// Enum representing the types of broadcasts that can be sent.
///
/// This enum is used to differentiate between different types of broadcasts
/// that can be sent from the environment to external entities.
///
/// Variants:
/// * `StopSignal`: Represents a signal to stop the event logger process.
// /// * `Event(Vec<Log>)`: Represents a broadcast of a vector of Ethereum logs.
#[derive(Clone, Debug)]
pub enum Broadcast {
    /// Represents a signal to stop the event logger process.
    StopSignal,
    /// Represents a broadcast of a vector of Ethereum logs.
    Event(Vec<String>, ReceiptData),
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const TEST_ENV_LABEL: &str = "test";
    const TEST_CONTRACT_SIZE_LIMIT: usize = 42069;
    const TEST_GAS_LIMIT: u64 = 1_333_333_333_337;

    #[test]
    fn new_with_parameters() {
        // let environment = Environment::builder()
        //     .with_label(TEST_ENV_LABEL)
        //     .with_contract_size_limit(TEST_CONTRACT_SIZE_LIMIT)
        //     .with_gas_limit(Felt::from(TEST_GAS_LIMIT));
        // assert_eq!(environment.parameters.label, Some(TEST_ENV_LABEL.into()));
        // assert_eq!(
        //     environment.parameters.contract_size_limit.unwrap(),
        //     TEST_CONTRACT_SIZE_LIMIT
        // );
        // assert_eq!(
        //     environment.parameters.gas_limit.unwrap(),
        //     Felt::from(TEST_GAS_LIMIT)
        // );
    }
}
