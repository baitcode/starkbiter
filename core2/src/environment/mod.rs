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

use starknet::core::types::{BlockId, BlockTag};
use starknet_devnet_core::constants::{
    self, ETH_ERC20_CONTRACT_ADDRESS, STRK_ERC20_CONTRACT_ADDRESS,
};

use starknet_devnet_core::starknet::{starknet_config::StarknetConfig, Starknet};
use starknet_devnet_core::state::{State, StateReader};
use starknet_devnet_types::felt::join_felts;
use starknet_devnet_types::num_bigint::BigUint;
use starknet_devnet_types::rpc::transaction_receipt::FeeInUnits;
use starknet_devnet_types::rpc::{
    gas_modification::GasModificationRequest, transaction_receipt::TransactionReceipt,
};

use starknet_devnet_types::starknet_api::core::{
    calculate_contract_address, ClassHash, ContractAddress, PatriciaKey,
};
use starknet_devnet_types::starknet_api::transaction::fields::{Calldata, ContractAddressSalt};
use starknet_types_core::felt::Felt;
use tokio::sync::broadcast::channel;

use super::*;

pub mod instruction;
use instruction::{
    Cheatcodes, CheatcodesReturn, EnvironmentData, Instruction, Outcome, ReceiptData,
    TxExecutionResult,
};

mod utils;

/// Alias for the sender of the channel for transmitting transactions.
pub(crate) type InstructionSender = Sender<Instruction>;

/// Alias for the receiver of the channel for transmitting transactions.
pub(crate) type InstructionReceiver = Receiver<Instruction>;

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
            // TODO: updat
            // let mut env = Env::default();
            // env.cfg.limit_contract_code_size = self.parameters.contract_size_limit;
            // env.block.gas_limit = self.parameters.gas_limit.unwrap_or(eU256::MAX);

            let mut starknet_config = &StarknetConfig::default();
            // Fork configuration

            let mut starknet = Starknet::new(starknet_config).unwrap();

            // Initialize counters that are returned on some receipts.
            let mut transaction_index = 0_u64;
            let mut last_executed_gas_price: u128 = 0;
            let mut cumulative_gas_per_block = BigUint::from(0_u128);

            // Loop over the instructions sent through the socket.
            while let Ok(instruction) = instruction_receiver.recv() {
                trace!(
                    "Instruction {:?} received by environment labeled: {:?}",
                    instruction,
                    label
                );
                match instruction {
                    Instruction::AddAccount {
                        public_key,
                        class_hash,
                        outcome_sender,
                    } => {
                        let mut state = starknet.get_state();

                        // TODO: check or predeclare?

                        let calldata = Calldata(Arc::new(vec![public_key]));
                        let deployer = ContractAddress::try_from(Felt::from(0_u32))
                            .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                        let account_address = calculate_contract_address(
                            ContractAddressSalt(Felt::from(20_u32)),
                            class_hash,
                            &calldata,
                            deployer,
                        )
                        .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                        // Predeploy the declared contract.
                        state
                            .set_class_hash_at(account_address, class_hash)
                            .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                        utils::mint_tokens_in_erc20_contract(
                            &mut state,
                            STRK_ERC20_CONTRACT_ADDRESS,
                            account_address.into(),
                            BigUint::from(100000_u128),
                        );

                        utils::simulate_constructor(state, account_address, public_key);

                        outcome_sender.send(Ok(Outcome::AddAccountCompleted(account_address)))?
                    }
                    Instruction::BlockUpdate {
                        block_number,
                        block_timestamp,
                        outcome_sender,
                    } => {
                        // TODO: skip it
                        // Return the old block data in a `ReceiptData`
                        let block = starknet.get_latest_block()?;

                        let receipt_data = ReceiptData {
                            block_number: block.block_number(),
                            transaction_index,
                            cumulative_gas_per_block: cumulative_gas_per_block.clone(),
                        };

                        // let state = starknet.pending_state;

                        // state.

                        // // Update the block number and timestamp
                        // evm.block_mut().number = U256::from_limbs(block_number.0);
                        // evm.block_mut().timestamp = U256::from_limbs(block_timestamp.0);

                        // Reset the counters.
                        transaction_index = 0_u64;
                        cumulative_gas_per_block = BigUint::from(0_u128);

                        // Return the old block data in a `ReceiptData` after the block update.
                        outcome_sender.send(Ok(Outcome::BlockUpdateCompleted(receipt_data)))?;
                    }
                    Instruction::Cheatcode {
                        cheatcode,
                        outcome_sender,
                    } => match cheatcode {
                        Cheatcodes::Load {
                            account,
                            key,
                            block, // TODO: Api Changed
                        } => {
                            match starknet.contract_storage_at_block(
                                &block,
                                account.into(),
                                key.0.into(),
                            ) {
                                Ok(value) => {
                                    outcome_sender.send(Ok(Outcome::CheatcodeReturn(
                                        CheatcodesReturn::Load { value },
                                    )))?;
                                }
                                Err(e) => {
                                    outcome_sender
                                        .send(Err(ArbiterCoreError::AccountDoesNotExistError))?;
                                }
                            }
                        }
                        Cheatcodes::Store {
                            account,
                            key,
                            value,
                        } => {
                            let mut state = starknet.get_state();
                            let result = state.set_storage_at(account, key, value);

                            // Mutate the db by inserting the new key-value pair into the account's
                            // storage and send the successful CheatcodeCompleted outcome.
                            match result {
                                Ok(account) => {
                                    outcome_sender.send(Ok(Outcome::CheatcodeReturn(
                                        CheatcodesReturn::Store,
                                    )))?;
                                }
                                Err(err) => {
                                    outcome_sender
                                        .send(Err(ArbiterCoreError::AccountDoesNotExistError))?;
                                }
                            };
                        }
                        Cheatcodes::Deal { address, amount } => {
                            let state = starknet.get_state();

                            utils::mint_tokens_in_erc20_contract(
                                state,
                                ETH_ERC20_CONTRACT_ADDRESS,
                                address.into(),
                                amount,
                            );

                            outcome_sender
                                .send(Ok(Outcome::CheatcodeReturn(CheatcodesReturn::Deal)))?;
                        }
                        Cheatcodes::Access { address } => {
                            // let &mut state = starknet.get_state();
                            // state.get_storage_at(contract_address, key);
                            let storage: HashMap<Felt, Felt> = HashMap::new();

                            outcome_sender.send(Ok(Outcome::CheatcodeReturn(
                                CheatcodesReturn::Access { storage },
                            )))?;
                        }
                    },
                    // A `Call` is not state changing and will not create events but will create
                    // console logs.
                    Instruction::Call {
                        call,
                        outcome_sender,
                    } => {
                        let latest = BlockId::Tag(BlockTag::Latest);

                        let res = starknet.call(
                            &latest,
                            call.contract_address.into(),
                            call.entry_point_selector,
                            call.calldata,
                        );
                        // TODO: Some errors should probably throw Err as Output.

                        // TODO: logs?

                        match res {
                            Ok(result) => {
                                outcome_sender.send(Ok(Outcome::CallCompleted(
                                    instruction::CallExecutionResult::Success(result),
                                )))?;
                            }
                            Err(e) => {
                                outcome_sender.send(Ok(Outcome::CallCompleted(
                                    // TODO: maybe pass error as is?
                                    instruction::CallExecutionResult::Failure(e.to_string()),
                                )))?;
                            }
                        }
                    }
                    Instruction::SetGasPrice {
                        gas_modification_request,
                        outcome_sender,
                    } => {
                        // TODO: discuss conversion
                        starknet.set_next_block_gas(gas_modification_request);
                        outcome_sender.send(Ok(Outcome::SetGasPriceCompleted))?;
                    }

                    // A `Transaction` is state changing and will create events.
                    Instruction::Transaction { tx, outcome_sender } => {
                        // Set the tx_env and prepare to process it

                        let result = &utils::execute_transaction(&mut starknet, tx)?;

                        let tx_receipt = match result {
                            TxExecutionResult::Revert(_, tx_receipt) => tx_receipt,
                            TxExecutionResult::Success(_, tx_receipt) => tx_receipt,
                        };

                        let receipt = match tx_receipt {
                            TransactionReceipt::Deploy(receipt) => &receipt.common,
                            TransactionReceipt::L1Handler(receipt) => &receipt.common,
                            TransactionReceipt::Common(receipt) => receipt,
                        };

                        // TODO: rethink cloning
                        last_executed_gas_price = match receipt.actual_fee.clone() {
                            FeeInUnits::WEI(fee_amount) => fee_amount.amount.0, // 1 ETH  = 10^9
                            FeeInUnits::FRI(fee_amount) => fee_amount.amount.0, // 1 STRK = 10^18
                        };

                        cumulative_gas_per_block += receipt.execution_resources.l2_gas;

                        let block_number = starknet
                            .get_block(&BlockId::Tag(BlockTag::Latest))
                            .unwrap()
                            .block_number();

                        let receipt_data = ReceiptData {
                            block_number,
                            transaction_index,
                            cumulative_gas_per_block: cumulative_gas_per_block.clone(),
                        };

                        // TODO: Logging
                        // TODO: Event broadcasting?
                        // TODO: Why we send Result<Outcome> instead of just Outcome?
                        outcome_sender.send(Ok(Outcome::TransactionCompleted(
                            result.clone(),
                            receipt_data,
                        )))?;
                        transaction_index += 1;
                    }
                    Instruction::Query {
                        environment_data,
                        outcome_sender,
                    } => {
                        let outcome = match environment_data {
                            EnvironmentData::BlockNumber => starknet
                                .get_latest_block()
                                .map_err(|_| ArbiterCoreError::InvalidQueryError)
                                .map(|block| block.block_number().0.to_be_bytes().to_vec())
                                .map(|bytes| Outcome::QueryReturn(bytes)),
                            EnvironmentData::BlockTimestamp => starknet
                                .get_latest_block()
                                .map_err(|_| ArbiterCoreError::InvalidQueryError)
                                .map(|block| block.timestamp().0.to_be_bytes().to_vec())
                                .map(|bytes| Outcome::QueryReturn(bytes)),
                            EnvironmentData::GasPrice => Ok(Outcome::QueryReturn(
                                last_executed_gas_price.to_be_bytes().to_vec(),
                            )),
                            EnvironmentData::Balance(address) => {
                                let token_address = ContractAddress::try_from(
                                    constants::ETH_ERC20_CONTRACT_ADDRESS,
                                )
                                .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                                // TODO: token symbol + classifier?
                                let balance = starknet
                                    .get_state()
                                    .get_fee_token_balance(address, token_address)
                                    .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                                Ok(Outcome::QueryReturn(
                                    join_felts(&balance.0, &balance.1).to_bytes_be(),
                                ))
                            }
                            EnvironmentData::Nonce(address) => {
                                let nonce = starknet
                                    .get_state()
                                    .get_nonce_at(address)
                                    .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

                                Ok(Outcome::QueryReturn(nonce.0.to_bytes_be().to_vec()))
                            }
                            EnvironmentData::Logs {} => todo!("FIND LOGS!"),
                        };
                        outcome_sender.send(outcome)?;
                    }
                    Instruction::Stop(outcome_sender) => {
                        match event_broadcaster.send(Broadcast::StopSignal) {
                            Ok(_) => {}
                            Err(_) => {
                                warn!("Stop signal was not sent to any listeners. Are there any listeners?")
                            }
                        }
                        // outcome_sender.send(Ok(Outcome::StopCompleted(db)))?;
                        outcome_sender.send(Ok(Outcome::StopCompleted()))?;
                        break;
                    }
                }
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
        let (outcome_sender, outcome_receiver) = bounded(1);
        self.socket
            .instruction_sender
            .send(Instruction::Stop(outcome_sender))?;
        let outcome = outcome_receiver.recv()??;

        let db = match outcome {
            // Outcome::StopCompleted(stopped_db) => stopped_db,
            Outcome::StopCompleted() => (),
            _ => unreachable!(),
        };

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
            .map_err(|_| ArbiterCoreError::JoinError)??;
        Ok(db)
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
