//! The [`middleware`] module provides functionality to interact with
//! Ethereum-like virtual machines. It achieves this by offering a middleware
//! implementation for sending and reading transactions, as well as watching
//! for events.
//!
//! Main components:
//! - [`ArbiterMiddleware`]: The core middleware implementation.
//! - [`Connection`]: Handles communication with the Ethereum VM.
//! - [`FilterReceiver`]: Facilitates event watching based on certain filters.

#![warn(missing_docs)]
use std::{pin::Pin, sync::Mutex};

use starknet::{
    core::{
        crypto::Signature,
        types::{BlockId, Felt},
    },
    providers::{JsonRpcClient, ProviderError},
    signers::{LocalWallet, Signer, SignerInteractivityContext, SigningKey, VerifyingKey},
};
use starknet_devnet_core::constants;
use starknet_devnet_types::{
    num_bigint::BigUint,
    rpc::{
        gas_modification::GasModificationRequest,
        transaction_receipt::TransactionReceipt,
        transactions::{BroadcastedTransaction, FunctionCall},
    },
    starknet_api::{
        block::{BlockNumber, BlockTimestamp},
        core::{ClassHash, ContractAddress},
        state::StorageKey,
    },
};

use super::environment::instruction::TxExecutionResult;
use super::*;
use crate::environment::{instruction::*, Broadcast, Environment};

pub mod connection;
use connection::*;

/// A middleware structure that integrates with `revm`.
///
/// [`ArbiterMiddleware`] serves as a bridge between the application and
/// [`revm`]'s execution environment, allowing for transaction sending, call
/// execution, and other core functions. It uses a custom connection and error
/// system tailored to Revm's specific needs.
///
/// This allows for [`revm`] and the [`Environment`] built around it to be
/// treated in much the same way as a live EVM blockchain can be addressed.
///
/// # Examples
///
/// Basic usage:
/// ```
/// use arbiter_core::{environment::Environment, middleware::ArbiterMiddleware};
///
/// // Create a new environment and run it
/// let mut environment = Environment::builder().build();
///
/// // Retrieve the environment to create a new middleware instance
/// let middleware = ArbiterMiddleware::new(&environment, Some("test_label"));
/// ```
/// The client can now be used for transactions with the environment.
/// Use a seed like `Some("test_label")` for maintaining a
/// consistent address across simulations and client labeling. Seeding is be
/// useful for debugging and post-processing.
#[derive(Debug)]
pub struct ArbiterMiddleware {
    connection: Connection,
    account_address: ContractAddress,
    account_signing_key: SigningKey,
    /// An optional label for the middleware instance
    #[allow(unused)]
    pub label: Option<String>,
}

impl ArbiterMiddleware {
    pub fn new(
        environment: &Environment,
        seed_and_label: Option<Felt>,
    ) -> Result<Arc<Self>, ArbiterCoreError> {
        let connection = Connection::from(environment);

        let signing_key = if let Some(seed) = seed_and_label {
            // TODO: probably will fail
            SigningKey::from_secret_scalar(seed)
        } else {
            SigningKey::from_random()
        };

        let pk = signing_key.verifying_key().scalar();

        // .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

        connection
            .instruction_sender
            .upgrade() // TODO: WHY?!
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::AddAccount {
                public_key: pk,
                class_hash: ClassHash(constants::ARGENT_CONTRACT_CLASS_HASH),
                outcome_sender: connection.outcome_sender.clone(),
            })?;

        let address = match connection.outcome_receiver.recv()?? {
            Outcome::AddAccountCompleted(address) => address,
            _ => unreachable!(),
        };

        info!(
            "Created new `ArbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
            environment.parameters.label
        );
        Ok(Arc::new(Self {
            account_address: address,
            account_signing_key: signing_key,
            connection,
            label: seed_and_label.map(|s| s.to_string()),
        }))
    }

    /// Provides access to the associated Ethereum provider which is given by
    /// the [`Provider<Connection>`] for [`ArbiterMiddleware`].
    fn provider(&self) -> &Connection {
        &self.connection
    }

    /// Allows the user to update the block number and timestamp of the
    /// [`Environment`] to whatever they may choose at any time.
    pub fn update_block(
        &self,
        block_number: impl Into<BlockNumber>,
        block_timestamp: impl Into<BlockTimestamp>,
    ) -> Result<ReceiptData, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::BlockUpdate {
                block_number: block_number.into(),
                block_timestamp: block_timestamp.into(),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::BlockUpdateCompleted(receipt_data) => Ok(receipt_data),
            _ => unreachable!(),
        }
    }

    /// Sends a cheatcode instruction to the environment.
    pub async fn apply_cheatcode(
        &self,
        cheatcode: Cheatcodes,
    ) -> Result<CheatcodesReturn, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Cheatcode {
                cheatcode,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::CheatcodeReturn(outcome) => Ok(outcome),
            _ => unreachable!(),
        }
    }

    /// Returns the address of the wallet/signer given to a client.
    /// Matches on the [`EOA`] variant of the [`ArbiterMiddleware`] struct.
    pub fn address(&self) -> ContractAddress {
        self.account_address
    }

    /// Allows a client to set a gas price for transactions.
    /// This can only be done if the [`Environment`] has
    /// [`EnvironmentParameters`] `gas_settings` field set to
    /// [`GasSettings::UserControlled`].
    pub async fn set_gas_price(
        &self,
        gas_modification_request: GasModificationRequest,
    ) -> Result<(), ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::SetGasPrice {
                gas_modification_request,
                outcome_sender: provider.outcome_sender.clone(),
            })?;
        match provider.outcome_receiver.recv()?? {
            Outcome::SetGasPriceCompleted => {
                debug!("Gas price set");
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    /// Returns a reference to the inner middleware of which there is none when
    /// using [`ArbiterMiddleware`] so we relink to `Self`
    fn inner(&self) -> &Connection {
        &self.connection
    }

    /// Provides the default sender address for transactions, i.e., the address
    /// of the wallet/signer given to a client of the [`Environment`].
    fn default_sender(&self) -> Option<ContractAddress> {
        Some(self.address())
    }

    /// Sends a transaction to the [`Environment`] which acts as a simulated
    /// Ethereum network.
    ///
    /// The method checks if the transaction is either a call to an existing
    /// contract or a deploy of a new one, and constructs the necessary
    /// transaction environment used for `revm`-based transactions.
    /// It then sends this transaction for execution and returns the
    /// corresponding pending transaction.
    async fn send_transaction<T: Into<BroadcastedTransaction> + Send + Sync>(
        &self,
        tx: T,
        _block: Option<BlockId>,
    ) -> Result<TransactionReceipt, ArbiterCoreError> {
        trace!("Building transaction");

        let instruction = Instruction::Transaction {
            tx: tx.into(),
            outcome_sender: self.provider().outcome_sender.clone(),
        };

        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(instruction)?;

        let outcome = provider.outcome_receiver.recv()??;

        if let Outcome::TransactionCompleted(execution_result, receipt_data) = outcome {
            match execution_result {
                TxExecutionResult::Success(res, tx_receipt) => Ok(tx_receipt),
                TxExecutionResult::Revert(reason, tx_receipt) => Ok(tx_receipt),
            }
        } else {
            unreachable!()
        }
    }

    /// Calls a contract method without creating a worldstate-changing
    /// transaction on the [`Environment`] (again, simulating the Ethereum
    /// network).
    ///
    /// Similar to `send_transaction`, this method checks if the call is
    /// targeting an existing contract or deploying a new one. After
    /// executing the call, it returns the output, but no worldstate change will
    /// be documented in the `revm` DB.
    async fn call(
        &self,
        call: FunctionCall,
        _block: Option<BlockId>,
    ) -> Result<Vec<Felt>, ArbiterCoreError> {
        trace!("Building call");

        let instruction = Instruction::Call {
            call,
            outcome_sender: self.provider().outcome_sender.clone(),
        };
        self.provider()
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(instruction)?;

        let outcome = self.provider().outcome_receiver.recv()??;

        if let Outcome::CallCompleted(execution_result) = outcome {
            match execution_result {
                CallExecutionResult::Success(felts) => Ok(felts),
                CallExecutionResult::Failure(_) => Err(ArbiterCoreError::CallError {}), // TODO: add data like reason and logs
            }
        } else {
            unreachable!()
        }
    }

    async fn get_gas_price(&self) -> Result<u128, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::GasPrice,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => {
                let bytes = outcome
                    .as_slice()
                    .try_into()
                    .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
                Ok(u128::from_be_bytes(bytes))
            }
            _ => unreachable!(),
        }
    }

    async fn get_block_number(&self) -> Result<u64, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::BlockNumber,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => {
                let bytes = outcome
                    .as_slice()
                    .try_into()
                    .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
                Ok(u64::from_be_bytes(bytes))
            }
            _ => unreachable!(),
        }
    }

    async fn get_balance<T: Into<ContractAddress> + Send + Sync>(
        &self,
        from: T,
        block: Option<BlockId>,
    ) -> Result<BigUint, ArbiterCoreError> {
        if block.is_some() {
            return Err(ArbiterCoreError::InvalidQueryError);
        }

        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::Balance(from.into()),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => {
                let bytes = outcome
                    .as_slice()
                    .try_into()
                    .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
                Ok(BigUint::from_bytes_be(bytes))
            }
            _ => unreachable!(),
        }
    }

    /// Returns the timestamp of the current block.
    pub async fn get_block_timestamp(&self) -> Result<u64, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::BlockTimestamp,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => {
                let bytes = outcome
                    .as_slice()
                    .try_into()
                    .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
                Ok(u64::from_be_bytes(bytes))
            }
            _ => unreachable!(),
        }
    }

    /// Returns the nonce of the address
    async fn get_nonce<T: Into<ContractAddress> + Send + Sync>(
        &self,
        from: T,
        _block: Option<BlockId>,
    ) -> Result<Felt, ArbiterCoreError> {
        let provider = self.provider();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::Nonce(from.into()),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => {
                let bytes = outcome
                    .as_slice()
                    .try_into()
                    .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
                Ok(Felt::from_bytes_be(bytes))
            }
            _ => unreachable!(),
        }
    }

    /// Fill necessary details of a transaction for dispatch
    ///
    /// This function is defined on providers to behave as follows:
    /// 1. populate the `from` field with the client address
    /// 2. Estimate gas usage
    ///
    /// It does NOT set the nonce by default.

    async fn fill_transaction(
        &self,
        tx: &mut BroadcastedTransaction,
        _block: Option<BlockId>,
    ) -> Result<(), ArbiterCoreError> {
        // Do nothing for now

        Ok(())
    }
    /// Fetches the value stored at the storage slot `key` for an account at
    /// `address`. todo: implement the storage at a specific block feature.
    async fn get_storage_at<T: Into<ContractAddress> + Send + Sync>(
        &self,
        account: T,
        key: StorageKey,
        block: BlockId,
    ) -> Result<Felt, ArbiterCoreError> {
        let result = self
            .apply_cheatcode(Cheatcodes::Load {
                account: account.into(),
                key,
                block,
            })
            .await
            .unwrap();

        match result {
            CheatcodesReturn::Load { value } => Ok(value),
            _ => unreachable!(),
        }
    }
}
