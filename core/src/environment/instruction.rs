//! This module contains the `Instruction` and `Outcome` enums that are used to
//! communicate instructions and their outcomes between the
//! [`middleware::ArbiterMiddleware`] and the [`Environment`].

use starknet::{
    core::types::{BlockId, Felt},
    signers::VerifyingKey,
};

use starknet_core::types as core_types;
use starknet_devnet_types::{
    felt::{Key, TransactionHash},
    num_bigint::BigUint,
    rpc::{
        transaction_receipt::TransactionReceipt,
        transactions::{BroadcastedTransaction, FunctionCall},
    },
    starknet_api::{
        block::{BlockNumber, BlockTimestamp},
        core::{ClassHash, ContractAddress},
        state::StorageKey,
    },
};

use super::*;

/// [`Instruction`]s that can be sent to the [`Environment`] via the
/// [`Socket`].
///
/// The [`Instruction`]s are sent to the [`Environment`] via the
///   [`Socket::instruction_sender`] and the results are received via the
///   [`crate::middleware::Connection::outcome_receiver`].
///
/// TODO: This is actually a ProviderRequestData from starknet-rs, they miss deserialise and partial_eq
#[derive(Debug, Clone)]
pub enum NodeInstruction {
    GetSpecVersion,
    GetBlockWithTxHashes {
        block_id: core_types::BlockId,
    },
    GetBlockWithTxs {
        block_id: core_types::BlockId,
    },
    GetBlockWithReceipts {
        block_id: core_types::BlockId,
    },
    GetStateUpdate {
        block_id: core_types::BlockId,
    },
    GetStorageAt {
        contract_address: core_types::Felt,
        key: core_types::Felt,
        block_id: core_types::BlockId,
    },
    GetMessagesStatus {
        transaction_hash: core_types::Hash256,
    },
    GetTransactionStatus {
        transaction_hash: core_types::Hash256,
    },
    GetTransactionByHash {
        transaction_hash: core_types::Hash256,
    },
    GetTransactionByBlockIdAndIndex {
        block_id: core_types::BlockId,
        index: u64,
    },
    GetTransactionReceipt {
        transaction_hash: core_types::Hash256,
    },
    GetClass {
        block_id: core_types::BlockId,
        class_hash: core_types::Felt,
    },
    GetClassHashAt {
        block_id: core_types::BlockId,
        contract_address: core_types::Felt,
    },
    GetClassAt {
        block_id: core_types::BlockId,
        contract_address: core_types::Felt,
    },
    GetBlockTransactionCount {
        block_id: core_types::BlockId,
    },
    Call {
        request: core_types::FunctionCall,
        block_id: core_types::BlockId,
    },
    EstimateFee {
        request: core_types::BroadcastedTransaction,
        simulate_flags: core_types::SimulationFlagForEstimateFee,
        block_id: core_types::BlockId,
    },
    EstimateMessageFee {
        message: core_types::MsgFromL1,
        block_id: core_types::BlockId,
    },
    BlockNumber,
    BlockHashAndNumber,
    ChainId,
    Syncing,
    GetEvents {
        filter: core_types::EventFilter,
        continuation_token: Option<String>,
        chunk_size: Option<u64>,
    },
    GetNonce {
        block_id: core_types::BlockId,
        contract_address: core_types::Felt,
    },
    AddInvokeTransaction {
        transaction: core_types::BroadcastedInvokeTransaction,
    },
    AddDeclareTransaction {
        transaction: core_types::BroadcastedDeclareTransaction,
    },
    AddDeployAccountTransaction {
        transaction: core_types::BroadcastedDeployAccountTransaction,
    },
    TraceTransaction {
        transaction_hash: TransactionHash,
    },
    SimulateTransactions {
        block_id: core_types::BlockId,
        transactions: Vec<core_types::BroadcastedTransaction>,
        simulation_flags: Vec<core_types::SimulationFlag>,
    },
    TraceBlockTransactions {
        block_id: core_types::BlockId,
    },
    // Not implemented
    //
    // GetStorageProof {
    //     block_id: core_types::ConfirmedBlockId,
    //     class_hashes: Vec<core_types::Felt>,
    //     contract_addresses: Vec<core_types::Felt>,
    //     contracts_storage_keys: Vec<core_types::ContractStorageKeys>,
    // },
    // ProviderRequest {
    //     request: ProviderRequestData,
    // },
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Node(NodeInstruction),
    Cheat(CheatInstruction),
    System,
}

/// TODO: This is actually a ProviderResponseData from starknet-rs, they miss deserialise and partial_eq
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeOutcome {
    SpecVersion(String),
    GetBlockWithTxHashes(core_types::MaybePendingBlockWithTxHashes),
    GetBlockWithTxs(core_types::MaybePendingBlockWithTxs),
    GetBlockWithReceipts(core_types::MaybePendingBlockWithReceipts),
    GetStateUpdate(core_types::MaybePendingStateUpdate),
    GetStorageAt(core_types::Felt),
    GetMessagesStatus(Vec<core_types::MessageWithStatus>),
    GetTransactionStatus(core_types::TransactionStatus),
    GetTransactionByHash(core_types::Transaction),
    GetTransactionByBlockIdAndIndex(core_types::Transaction),
    GetTransactionReceipt(core_types::TransactionReceiptWithBlockInfo),
    GetClass(core_types::ContractClass),
    GetClassHashAt(core_types::Felt),
    GetClassAt(core_types::ContractClass),
    GetBlockTransactionCount(u64),
    Call(Vec<core_types::Felt>),
    EstimateFee(Vec<core_types::FeeEstimate>),
    EstimateMessageFee(core_types::FeeEstimate),
    BlockNumber(u64),
    BlockHashAndNumber(core_types::BlockHashAndNumber),
    ChainId(core_types::Felt),
    Syncing(core_types::SyncStatusType),
    GetEvents(core_types::EventsPage),
    GetNonce(core_types::Felt),
    GetStorageProof(core_types::StorageProof),
    AddInvokeTransaction(core_types::InvokeTransactionResult),
    AddDeclareTransaction(core_types::DeclareTransactionResult),
    AddDeployAccountTransaction(core_types::DeployAccountTransactionResult),
    TraceTransaction(core_types::TransactionTrace),
    SimulateTransactions(Vec<core_types::SimulatedTransaction>),
    TraceBlockTransactions(Vec<core_types::TransactionTraceWithHash>),
}

/// [`Outcome`]s that can be sent back to the the client via the
/// [`Socket`].
/// These outcomes can be from `Call`, `Transaction`, or `BlockUpdate`
/// instructions sent to the [`Environment`]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    Node(NodeOutcome),
    Cheat(CheatcodesReturn),
}

type GasUsed = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TxExecutionResult {
    /// The transaction was successful and the outcome is an `ExecutionResult`.
    Success(TransactionHash, TransactionReceipt),
    /// The transaction failed and the outcome is a `String` revert reason.
    Revert(String, TransactionReceipt),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CallExecutionResult {
    /// The call was successful and the outcome is a vector of `Felt` values.
    Success(Vec<Felt>),
    /// The call failed and the outcome is a `String` revert reason.
    Failure(String),
}

/// [`EnvironmentData`] is an enum used inside of the [`Instruction::Query`] to
/// specify what data should be returned to the user.
/// Currently this may be the block number, block timestamp, gas price, or
/// balance of an account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum EnvironmentData {
    /// The query is for the block number of the [`EVM`].
    BlockNumber,

    /// The query is for the block timestamp of the [`EVM`].
    BlockTimestamp,

    /// The query is for the gas price of the [`EVM`].
    GasPrice,

    /// The query is for the balance of an account given by the inner `Address`.
    Balance(ContractAddress),

    /// The query is for the nonce of an account given by the inner `Address`.
    Nonce(ContractAddress),

    /// Query for logs in a range of blocks.
    Logs {
        // filter: EventFilter,
    },
}

/// [`ReceiptData`] is a structure that holds the block number, transaction
/// index, and cumulative gas used per block for a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptData {
    /// `block_number` is the number of the block in which the transaction was
    /// included.
    pub block_number: BlockNumber,
    /// `transaction_index` is the index position of the transaction in the
    /// block.
    pub transaction_index: u64,
    /// `cumulative_gas_per_block` is the total amount of gas used in the
    /// block up until and including the transaction.
    pub cumulative_gas_per_block: BigUint,
}

/// Cheatcodes are a direct way to access the underlying [`EVM`] environment and
/// database.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cheatcodes {
    /// A `Deal` is used to increase the balance of an account in the [`EVM`].
    Deal {
        /// The address of the account to increase the balance of.
        address: ContractAddress,

        /// The amount to increase the balance of the account by.
        amount: BigUint,
    },
    /// Fetches the value of a storage slot of an account.
    Load {
        /// The address of the account to fetch the storage slot from.
        address: Felt,
        /// The storage slot to fetch.
        key: Felt,
        /// The block to fetch the storage slot from.
        /// todo: implement storage slots at blocks.
        block: BlockId,
    },
    /// Overwrites a storage slot of an account.
    /// TODO: for more complicated data types, like structs, there's more work
    /// to do.
    Store {
        /// The address of the account to overwrite the storage slot of.
        account: ContractAddress,
        /// The storage slot to overwrite.
        key: StorageKey,
        /// The value to overwrite the storage slot with.
        value: Felt,
    },
    /// Fetches the `DbAccount` account at the given address.
    Access {
        /// The address of the account to fetch.
        address: Felt,
    },
}

/// Wrapper around [`AccountState`] that can be serialized and deserialized.
#[derive(Debug, Clone, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AccountStateSerializable {
    /// Before Spurious Dragon hardfork there was a difference between empty and
    /// not existing. And we are flagging it here.
    NotExisting,
    /// EVM touched this account. For newer hardfork this means it can be
    /// cleared/removed from state.
    Touched,
    /// EVM cleared storage of this account, mostly by selfdestruct, we don't
    /// ask database for storage slots and assume they are U256::ZERO
    StorageCleared,
    /// EVM didn't interacted with this account
    #[default]
    None,
}

#[derive(Debug, Clone)]
pub enum CheatInstruction {
    CreateAccount {
        public_key: VerifyingKey,
        class_hash: Felt,
    },
    CreateBlock,
}
/// Return values of applying cheatcodes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CheatcodesReturn {
    CreateAccount(ContractAddress),
    CreateBlock,
    // A `Store` returns nothing.
    // Store,
    // A `Deal` returns nothing.
    // Deal,
    // Gets the DbAccount associated with an address.
    // Access {
    //     /// Basic account information like nonce, balance, code hash, bytcode.
    //     // info: AccountInfo,
    //     /// todo: revm must be updated with serde deserialize, then `DbAccount`
    //     /// can be used.
    //     // account_state: AccountStateSerializable,
    //     /// Storage slots of the account.
    //     storage: HashMap<Felt, Felt>,
    // },
}
