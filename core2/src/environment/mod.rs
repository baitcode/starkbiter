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
// use ethers::{abi::AbiDecode, types::ValueOrArray};
// use revm::{
//     db::AccountState,
//     inspector_handle_register,
//     primitives::{Env, HashMap, B256},
// };
use sha2::digest::block_buffer::Block;
use starknet::core::types::{BlockId, BlockTag, BroadcastedTransaction, Felt, TransactionReceipt};
use starknet::core::utils::get_storage_var_address;
use starknet_devnet_core::account::Account;
use starknet_devnet_core::constants::{self, ETH_ERC20_CONTRACT_ADDRESS};
use starknet_devnet_core::starknet::{starknet_config::StarknetConfig, Starknet};
use starknet_devnet_core::state::{State, StateReader};
use starknet_devnet_types::felt::{join_felts, split_biguint};
use starknet_devnet_types::rpc::gas_modification::GasModificationRequest;
use tokio::sync::broadcast::channel;

use super::*;
use crate::middleware::connection::revm_logs_to_ethers_logs;
#[cfg_attr(doc, doc(hidden))]
#[cfg_attr(doc, allow(unused_imports))]
#[cfg(doc)]
use crate::middleware::ArbiterMiddleware;

pub mod instruction;
use instruction::*;

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
    pub gas_limit: Option<eU256>,

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
    pub fn with_gas_limit(mut self, gas_limit: eU256) -> Self {
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
        let db = self.db.clone();

        // Bring in the inspector
        let inspector = self.inspector.take().unwrap();

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

            let starknet = Starknet::new(starknet_config).unwrap();

            // Initialize counters that are returned on some receipts.
            let mut transaction_index = 0_u64;
            let mut last_executed_tx_gas = eU256::from(0);
            let mut cumulative_gas_per_block = eU256::from(0);

            // Loop over the instructions sent through the socket.
            while let Ok(instruction) = instruction_receiver.recv() {
                trace!(
                    "Instruction {:?} received by environment labeled: {:?}",
                    instruction,
                    label
                );
                match instruction {
                    Instruction::AddAccount {
                        address,
                        outcome_sender,
                    } => {
                        let recast_address = eAddress::from(address.as_fixed_bytes());
                        // TODO: weird account operation. discuss with team.

                        // match db.state.write()?.accounts.insert(recast_address, account) {
                        //     None => outcome_sender.send(Ok(Outcome::AddAccountCompleted))?,
                        //     Some(_) => {
                        //         outcome_sender.send(Err(ArbiterCoreError::AccountCreationError))?;
                        //     }
                        // }
                    }
                    Instruction::BlockUpdate {
                        block_number,
                        block_timestamp,
                        outcome_sender,
                    } => {
                        // TODO: skip it
                        // Return the old block data in a `ReceiptData`
                        let block = starknet.get_block(BlockId::Tag(BlockTag::Latest))?;

                        let receipt_data = ReceiptData {
                            block_number: block.block_number(),
                            transaction_index,
                            cumulative_gas_per_block,
                        };

                        let &mut state = starknet.pending_state;

                        // state.

                        // // Update the block number and timestamp
                        // evm.block_mut().number = U256::from_limbs(block_number.0);
                        // evm.block_mut().timestamp = U256::from_limbs(block_timestamp.0);

                        // Reset the counters.
                        transaction_index = u64::from(0);
                        cumulative_gas_per_block = eU256::from(0);

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
                        } => match starknet.contract_storage_at_block(block, account, key) {
                            Ok(value) => {
                                outcome_sender.send(Ok(Outcome::CheatcodeReturn(
                                    CheatcodesReturn::Load { value },
                                )))?;
                            }
                            Err(e) => {
                                outcome_sender
                                    .send(Err(ArbiterCoreError::AccountDoesNotExistError))?;
                            }
                        },
                        Cheatcodes::Store {
                            account,
                            key,
                            value,
                        } => {
                            let &mut state = starknet.get_state();
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
                            let recast_address = eAddress::from(address.as_fixed_bytes());
                            let state = starknet.get_state();

                            // TODO: add support for other tokens
                            let token_address = ETH_ERC20_CONTRACT_ADDRESS;

                            let balance_var_key_low =
                                get_storage_var_address("ERC20_balances", &[Felt::from(address)])?
                                    .try_into()?;
                            let balance_var_key_high = balance_var_key_low.next_storage_key()?;

                            let current_value = join_felts(
                                state.get_storage_at(token_address, balance_var_key_high),
                                state.get_storage_at(token_address, balance_var_key_low),
                            );

                            let total_supply_key_low =
                                get_storage_var_address("ERC20_total_supply", &[])?.try_into()?;
                            let total_supply_key_high = total_supply_key_low.next_storage_key()?;

                            let total_supply_low =
                                state.get_storage_at(token_address, total_supply_key_low)?;
                            let total_supply_high =
                                state.get_storage_at(token_address, total_supply_key_high)?;

                            let new_total_supply =
                                join_felts(&total_supply_high, &total_supply_low) + amount;

                            let (new_total_supply_high, new_total_supply_low) =
                                split_biguint(new_total_supply);

                            let (new_high, new_low) = split_biguint(current_value + amount);

                            state.set_storage_at(token_address, balance_var_key_low, new_low)?;
                            state.set_storage_at(token_address, balance_var_key_high, new_high)?;

                            outcome_sender
                                .send(Ok(Outcome::CheatcodeReturn(CheatcodesReturn::Deal)))?;
                        }
                        Cheatcodes::Access { address } => {
                            let &mut state = starknet.get_state();
                            // state.get_storage_at(contract_address, key);
                            let storage: HashMap<Felt, Felt> = HashMap::new();

                            outcome_sender.send(Ok(Outcome::CheatcodeReturn(storage)))?;
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
                            latest,
                            call.contract_address,
                            call.entry_point_selector,
                            call.calldata,
                        );

                        match res {
                            Ok(result) => {
                                // logs?
                                outcome_sender.send(Ok(Outcome::CallCompleted(result)))?;
                            }
                            Err(e) => {
                                outcome_sender.send(Err(ArbiterCoreError::DevnetError(e)))?;
                            }
                        }
                    }
                    Instruction::SetGasPrice {
                        gas_price,
                        outcome_sender,
                    } => {
                        // TODO: discuss conversion
                        let gas_modification_request = GasModificationRequest {
                            gas_price_wei: 0,
                            data_gas_price_wei: 0,
                            gas_price_fri: 0,
                            data_gas_price_fri: 0,
                            l2_gas_price_wei: 0,
                            l2_gas_price_fri: 0,
                            generate_block: 0,
                        };
                        starknet.set_next_block_gas(gas_modification_request);
                        outcome_sender.send(Ok(Outcome::SetGasPriceCompleted))?;
                    }

                    // A `Transaction` is state changing and will create events.
                    Instruction::Transaction { tx, outcome_sender } => {
                        // Set the tx_env and prepare to process it
                        let outcome = match tx {
                            BroadcastedTransaction::Invoke(tx) => {
                                match starknet.add_invoke_transaction(tx) {
                                    Ok(transaction_hash) => {
                                        let receipt = starknet
                                            .get_transaction_receipt_by_hash(&transaction_hash)?;
                                        let res = TransactionResult::InvokeResult(transaction_hash);
                                        if let TransactionReceipt::Invoke(r) = receipt {
                                            last_executed_tx_gas = r.execution_resources.l2_gas;
                                            cumulative_gas_per_block += last_executed_tx_gas;
                                        } else {
                                            panic!("Unexpected non Invoke transaction receipt");
                                        }
                                        Ok(Outcome::TransactionCompleted(res, receipt))
                                    }
                                    Err(e) => Err(ArbiterCoreError::DevnetError(e)),
                                }
                            }
                            BroadcastedTransaction::DeployAccount(tx) => {
                                match starknet.add_deploy_account_transaction(tx) {
                                    Ok((transaction_hash, contract_address)) => {
                                        let receipt = starknet
                                            .get_transaction_receipt_by_hash(&transaction_hash)?;
                                        let res = TransactionResult::DeployResult(
                                            transaction_hash,
                                            contract_address,
                                        );
                                        if let TransactionReceipt::Deploy(r) = receipt {
                                            last_executed_tx_gas = r.execution_resources.l2_gas;
                                            cumulative_gas_per_block += last_executed_tx_gas;
                                        } else {
                                            panic!("Unexpected non Deploy transaction receipt");
                                        }
                                        Ok(Outcome::TransactionCompleted(res, receipt))
                                    }
                                    Err(e) => Err(ArbiterCoreError::DevnetError(e)),
                                }
                            }
                            BroadcastedTransaction::Declare(tx) => {
                                match starknet.add_declare_transaction(tx) {
                                    Ok((transaction_hash, class_hash)) => {
                                        let receipt = starknet
                                            .get_transaction_receipt_by_hash(&transaction_hash)?;
                                        let res = TransactionResult::DeclareResult(
                                            transaction_hash,
                                            class_hash,
                                        );
                                        if let TransactionReceipt::Declare(r) = receipt {
                                            last_executed_tx_gas = r.execution_resources.l2_gas;
                                            cumulative_gas_per_block += last_executed_tx_gas;
                                        } else {
                                            panic!("Unexpected non Deploy transaction receipt");
                                        }
                                        Ok(Outcome::TransactionCompleted(res, receipt));
                                    }
                                    Err(e) => {
                                        Err(ArbiterCoreError::DevnetError(e));
                                    }
                                }
                            }
                        };

                        let block_number = starknet
                            .get_block(BlockId::Tag(BlockTag::Latest))
                            .unwrap()
                            .block_number();
                        let receipt_data = ReceiptData {
                            block_number,
                            transaction_index,
                            cumulative_gas_per_block,
                        };

                        // TODO: Logging
                        // TODO: Event broadcasting?

                        outcome_sender.send(outcome)?;
                        transaction_index += 1;
                    }
                    Instruction::Query {
                        environment_data,
                        outcome_sender,
                    } => {
                        let outcome = match environment_data {
                            EnvironmentData::BlockNumber => match starknet.get_latest_block() {
                                Ok(block) => {
                                    Ok(Outcome::QueryReturn(block.block_number().0.to_string()))
                                }
                                Err(e) => {
                                    return Err(ArbiterCoreError::InvalidQueryError(e));
                                }
                            },
                            EnvironmentData::BlockTimestamp => match starknet.get_latest_block() {
                                Ok(block) => {
                                    Ok(Outcome::QueryReturn(block.timestamp().0.to_string()))
                                }
                                Err(e) => {
                                    return Err(ArbiterCoreError::InvalidQueryError(e));
                                }
                            },
                            EnvironmentData::GasPrice => {
                                Ok(Outcome::QueryReturn(last_executed_tx_gas.to_string()))
                            }
                            EnvironmentData::Balance(address) => {
                                // TODO: token symbol + classifier?
                                let balance = starknet.get_state().get_fee_token_balance(
                                    address,
                                    constants::ETH_ERC20_CONTRACT_ADDRESS,
                                );
                                match balance {
                                    Err(e) => {
                                        return Err(ArbiterCoreError::InvalidQueryError(e));
                                    }
                                    Ok((low, high)) => {
                                        return Ok(Outcome::QueryReturn(
                                            join_felts(&high, &low).to_str_radix(10),
                                        ));
                                    }
                                }
                            }
                            EnvironmentData::TransactionCount(address) => {
                                let nonce = starknet.get_state().get_nonce_at(address);
                                match nonce {
                                    Err(e) => {
                                        return Err(ArbiterCoreError::InvalidQueryError(e));
                                    }
                                    Ok(nonce) => {
                                        return Ok(Outcome::QueryReturn(nonce.to_string()));
                                    }
                                }
                            }
                            EnvironmentData::Logs { filter } => todo!("FIND LOGS!"),
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
        let environment = Environment::builder()
            .with_label(TEST_ENV_LABEL)
            .with_contract_size_limit(TEST_CONTRACT_SIZE_LIMIT)
            .with_gas_limit(Felt::from(TEST_GAS_LIMIT));
        assert_eq!(environment.parameters.label, Some(TEST_ENV_LABEL.into()));
        assert_eq!(
            environment.parameters.contract_size_limit.unwrap(),
            TEST_CONTRACT_SIZE_LIMIT
        );
        assert_eq!(
            environment.parameters.gas_limit.unwrap(),
            Felt::from(TEST_GAS_LIMIT)
        );
    }
}
