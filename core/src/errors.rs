//! Errors that can occur when managing or interfacing with Starkbiter's
//! sandboxed Starknet Devnet environment.

use std::sync::{PoisonError, RwLockWriteGuard};

use crossbeam_channel::{RecvError, SendError};
use starknet_devnet_core::error::Error;
use thiserror::Error;

use self::environment::instruction::{Instruction, Outcome};
use super::*;

/// The error type for `starkbiter-core`.
#[derive(Error, Debug)]
pub enum StarkbiterCoreError {
    /// Tried to create an account that already exists.
    #[error("Account already exists!")]
    AccountCreationError,

    /// Tried to fetch data from underlying fork.
    #[error("No forking config exists!")]
    NoForkConfig,

    /// Failed to calculate the account address.
    #[error("Can't calculate account address!")]
    AccountAddressError,

    /// Tried to access an account that doesn't exist.
    #[error("Account doesn't exist!")]
    AccountDoesNotExistError,

    /// Tried to sign with forked EOA.
    #[error("Can't sign with a forked EOA!")]
    ForkedEOASignError,

    /// Failed to upgrade instruction sender in middleware.
    #[error("Failed to upgrade sender to a strong reference!")]
    UpgradeSenderError,

    /// Data missing when calling a transaction.
    #[error("Data missing when calling a transaction!")]
    MissingDataError,

    /// Invalid data used for a query request.
    #[error("Invalid data used for a query request!")]
    InvalidQueryError,

    /// Failed to join environment thread on stop.
    #[error("Failed to join environment thread on stop!")]
    JoinError,

    /// Reverted execution.
    #[error("Execution failed with revert: {gas_used:?} gas used, {output:?}")]
    ExecutionRevert {
        /// The amount of gas used.
        gas_used: u64,
        /// The output bytes of the execution.
        output: String,
    },

    /// Halted execution.
    #[error("Call failed.")]
    CallError {},

    /// Failed to parse integer.
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),

    /// Evm had a runtime error.
    #[error(transparent)]
    DevnetError(#[from] Error),

    /// Send error.
    #[error(transparent)]
    SendError(
        #[from]
        #[allow(private_interfaces)]
        Box<SendError<Instruction>>,
    ),

    /// Recv error.
    #[error(transparent)]
    RecvError(#[from] RecvError),

    /// Failed to handle json.
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    /// Failed to reply to instruction.
    #[error("{0}")]
    ReplyError(String),

    /// Failed to grab a lock.
    #[error("{0}")]
    RwLockError(String),

    /// Represents an internal error with a message.
    /// Used to wrap all errors without specific wrapper
    // TODO: remove and make more specific errors.
    #[error("{0}")]
    InternalError(String),
}

impl From<SendError<Result<Outcome, StarkbiterCoreError>>> for StarkbiterCoreError {
    fn from(e: SendError<Result<Outcome, StarkbiterCoreError>>) -> Self {
        StarkbiterCoreError::ReplyError(e.to_string())
    }
}

impl<T> From<PoisonError<RwLockWriteGuard<'_, T>>> for StarkbiterCoreError {
    fn from(e: PoisonError<RwLockWriteGuard<'_, T>>) -> Self {
        StarkbiterCoreError::RwLockError(e.to_string())
    }
}
