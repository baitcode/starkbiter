//! This module contains the [`Instruction`] and [`Outcome`] enums used to
//! communicate instructions and their outcomes between the
//! [`middleware::StarkbiterMiddleware`] and the [`Environment`].

use starknet::{core::types::Felt, signers::SigningKey};
use starknet_core::types as core_types;
use starknet_devnet_types::{
    felt::TransactionHash,
    num_bigint::BigUint,
    rpc::{
        gas_modification::GasModification, transaction_receipt::TransactionReceipt,
        transactions::l1_handler_transaction::L1HandlerTransaction,
    },
    starknet_api::block::BlockNumber,
};

use super::*;
use crate::tokens::TokenId;

/// Filter to pass to replay block method so that only transactions containing
/// certain events would be applied. This is merely an optimisation, as block
/// replay with 200 tx would take 20 mins.
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Matched events emitted by this address
    pub from_address: Felt,
    /// Matches events against keys list: vec![vec![selector, key1, key2]]
    /// selector is a keccak hash from event name.
    pub keys: Vec<Vec<Felt>>,
}

/// Instructions that can be sent to the [`Environment`] via the [`Socket`].
///
/// These instructions are sent to the [`Environment`] via the
/// [`Socket::instruction_sender`] and the results are received via the
/// [`crate::middleware::Connection::outcome_receiver`].
///
/// TODO: This is actually a ProviderRequestData from starknet-rs, but they lack
/// `Deserialize` and `PartialEq`.
#[derive(Debug, Clone)]
pub enum NodeInstruction {
    /// Gets the specification version of the node. Returns a constant
    /// "unknown".
    GetSpecVersion,

    /// Gets the block with transaction hashes for the given block id. Mirrors
    /// node RPC API.
    GetBlockWithTxHashes {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Gets the block with full transactions for the given block id. Mirrors
    /// node RPC API.
    GetBlockWithTxs {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Gets the block with receipts for the given block id. Mirrors node RPC
    /// API.
    GetBlockWithReceipts {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Gets the state update for the given block id. Mirrors node RPC API.
    GetStateUpdate {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Gets the value of a storage slot at a given contract address and key for
    /// a specific block.
    GetStorageAt {
        /// The address of the contract to retrieve the storage slot from.
        contract_address: core_types::Felt,
        /// The key of the storage slot to retrieve.
        key: core_types::Felt,
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Gets the status of messages for a given transaction hash.
    GetMessagesStatus {
        /// The transaction hash to query.
        transaction_hash: core_types::Hash256,
    },
    /// Gets the status of a transaction for a given transaction hash.
    GetTransactionStatus {
        /// The transaction hash to query.
        transaction_hash: core_types::Hash256,
    },
    /// Gets the transaction by its hash.
    GetTransactionByHash {
        /// The transaction hash to query.
        transaction_hash: core_types::Hash256,
    },
    /// Gets the transaction by block id and index.
    GetTransactionByBlockIdAndIndex {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The index of the transaction in the block.
        index: u64,
    },
    /// Gets the transaction receipt for a given transaction hash.
    GetTransactionReceipt {
        /// The transaction hash to query.
        transaction_hash: core_types::Hash256,
    },
    /// Gets the class for a given block id and class hash.
    GetClass {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The class hash to query.
        class_hash: core_types::Felt,
    },
    /// Gets the class hash at a given block id and contract address.
    GetClassHashAt {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The contract address to query.
        contract_address: core_types::Felt,
    },
    /// Gets the class at a given block id and contract address.
    GetClassAt {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The contract address to query.
        contract_address: core_types::Felt,
    },
    /// Gets the number of transactions in a block.
    GetBlockTransactionCount {
        /// The identifier of the block.
        block_id: core_types::BlockId,
    },
    /// Calls a contract function.
    Call {
        /// The function call request.
        request: core_types::FunctionCall,
        /// The identifier of the block.
        block_id: core_types::BlockId,
    },
    /// Estimates the fee for a transaction.
    EstimateFee {
        /// The transaction to estimate the fee for.
        request: core_types::BroadcastedTransaction,
        /// Simulation flags for fee estimation.
        simulate_flags: Vec<core_types::SimulationFlagForEstimateFee>,
        /// The identifier of the block.
        block_id: core_types::BlockId,
    },
    /// Estimates the fee for a message from L1.
    EstimateMessageFee {
        /// The message from L1.
        message: core_types::MsgFromL1,
        /// The identifier of the block.
        block_id: core_types::BlockId,
    },
    /// Gets the current block number.
    BlockNumber,
    /// Gets the current block hash and number.
    BlockHashAndNumber,
    /// Gets the chain ID.
    ChainId,
    /// Gets the syncing status.
    Syncing,
    /// Gets events matching the given filter.
    GetEvents {
        /// The event filter.
        filter: core_types::EventFilter,
        /// The continuation token for pagination.
        continuation_token: Option<String>,
        /// The chunk size for pagination.
        chunk_size: Option<u64>,
    },
    /// Gets the nonce for a contract at a given block.
    GetNonce {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The contract address to query.
        contract_address: core_types::Felt,
    },
    /// Adds an invoke transaction.
    AddInvokeTransaction {
        /// The transaction to add.
        transaction: core_types::BroadcastedInvokeTransaction,
    },
    /// Adds a declare transaction.
    AddDeclareTransaction {
        /// The transaction to add.
        transaction: core_types::BroadcastedDeclareTransaction,
    },
    /// Adds a deploy account transaction.
    AddDeployAccountTransaction {
        /// The transaction to add.
        transaction: core_types::BroadcastedDeployAccountTransaction,
    },
    /// Traces a transaction by its hash.
    TraceTransaction {
        /// The transaction hash to trace.
        transaction_hash: TransactionHash,
    },
    /// Simulates a list of transactions.
    SimulateTransactions {
        /// The identifier of the block.
        block_id: core_types::BlockId,
        /// The transactions to simulate.
        transactions: Vec<core_types::BroadcastedTransaction>,
        /// Simulation flags.
        simulation_flags: Vec<core_types::SimulationFlag>,
    },
    /// Traces all transactions in a block.
    TraceBlockTransactions {
        /// The identifier of the block.
        block_id: core_types::BlockId,
    },
    // Not implemented:
    // GetStorageProof { ... }
    // ProviderRequest { ... }
}

/// Instruction wrapper for node, cheat, or system instructions.
#[derive(Debug, Clone)]
pub enum Instruction {
    /// Node-related instruction.
    Node(NodeInstruction),
    /// Cheatcode-related instruction.
    Cheat(CheatInstruction),
    /// System instruction.
    System(SystemInstruction),
}

/// Represents system-level instructions that can be sent to the [`Environment`]
/// via the [`Socket`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemInstruction {
    /// Stops the environment, stops listening for new events.
    Stop,
}

/// Represents the possible outcomes returned from processing
/// [`SystemInstruction`]s.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemInstructionOutcome {
    /// Indicates stop was successful.
    Stop,
}

/// Represents the possible outcomes returned from processing
/// [`NodeInstruction`]s.
///
/// Each variant corresponds to the result of a specific node instruction,
/// mirroring the Starknet node RPC API. These outcomes are used to communicate
/// results from the environment back to the middleware or client.
///
/// Many variants wrap types from `starknet_core::types`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeOutcome {
    /// The specification version of the node.
    SpecVersion(String),
    /// The block with transaction hashes, or a pending block.
    GetBlockWithTxHashes(Box<core_types::MaybePendingBlockWithTxHashes>),
    /// The block with full transactions, or a pending block.
    GetBlockWithTxs(Box<core_types::MaybePendingBlockWithTxs>),
    /// The block with receipts, or a pending block.
    GetBlockWithReceipts(Box<core_types::MaybePendingBlockWithReceipts>),
    /// The state update for a given block, or a pending state update.
    GetStateUpdate(Box<core_types::MaybePendingStateUpdate>),
    /// The value of a storage slot.
    GetStorageAt(core_types::Felt),
    /// The status of L1/L2 messages for a transaction.
    GetMessagesStatus(Vec<core_types::MessageWithStatus>),
    /// The status of a transaction.
    GetTransactionStatus(core_types::TransactionStatus),
    /// The transaction details by hash.
    GetTransactionByHash(Box<core_types::Transaction>),
    /// The transaction details by block id and index.
    GetTransactionByBlockIdAndIndex(Box<core_types::Transaction>),
    /// The transaction receipt with block info.
    GetTransactionReceipt(Box<core_types::TransactionReceiptWithBlockInfo>),
    /// The contract class for a given class hash.
    GetClass(Box<core_types::ContractClass>),
    /// The class hash at a contract address.
    GetClassHashAt(core_types::Felt),
    /// The contract class at a contract address.
    GetClassAt(Box<core_types::ContractClass>),
    /// The number of transactions in a block.
    GetBlockTransactionCount(u64),
    /// The result of a contract call.
    Call(Vec<core_types::Felt>),
    /// The estimated fee(s) for a transaction.
    EstimateFee(Vec<core_types::FeeEstimate>),
    /// The estimated fee for a message from L1.
    EstimateMessageFee(Box<core_types::FeeEstimate>),
    /// The current block number.
    BlockNumber(u64),
    /// The current block hash and number.
    BlockHashAndNumber(core_types::BlockHashAndNumber),
    /// The chain ID.
    ChainId(core_types::Felt),
    /// The syncing status of the node.
    Syncing(Box<core_types::SyncStatusType>),
    /// The page of events matching a filter.
    GetEvents(core_types::EventsPage),
    /// The nonce for a contract.
    GetNonce(core_types::Felt),
    /// The storage proof for a contract and key.
    GetStorageProof(Box<core_types::StorageProof>),
    /// The result of adding an invoke transaction.
    AddInvokeTransaction(core_types::InvokeTransactionResult),
    /// The result of adding a declare transaction.
    AddDeclareTransaction(core_types::DeclareTransactionResult),
    /// The result of adding a deploy account transaction.
    AddDeployAccountTransaction(core_types::DeployAccountTransactionResult),
    /// The trace of a transaction.
    TraceTransaction(Box<core_types::TransactionTrace>),
    /// The result of simulating transactions.
    SimulateTransactions(Vec<core_types::SimulatedTransaction>),
    /// The traces of all transactions in a block.
    TraceBlockTransactions(Vec<core_types::TransactionTraceWithHash>),
}

/// Outcomes that can be sent back to the client via the [`Socket`].
///
/// These outcomes can be from `Call`, `Transaction`, or `BlockUpdate`
/// instructions sent to the [`Environment`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Outcome {
    /// Node-related outcome.
    Node(NodeOutcome),
    /// Cheatcode-related outcome.
    Cheat(CheatcodesOutcome),
    /// System-related outcome.
    System(SystemInstructionOutcome),
}

/// The result of executing a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TxExecutionResult {
    /// The transaction was successful and the outcome is an
    /// [`ExecutionResult`].
    Success(TransactionHash, TransactionReceipt),
    /// The transaction failed and the outcome is a revert reason.
    Revert(String, TransactionReceipt),
}

/// The result of executing a call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CallExecutionResult {
    /// The call was successful and the outcome is a vector of [`Felt`] values.
    Success(Vec<Felt>),
    /// The call failed and the outcome is a revert reason.
    Failure(String),
}

/// [`ReceiptData`] holds the block number, transaction index, and cumulative
/// gas used per block for a transaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReceiptData {
    /// The number of the block in which the transaction was included.
    pub block_number: BlockNumber,
    /// The index position of the transaction in the block.
    pub transaction_index: u64,
    /// The total amount of gas used in the block up until and including the
    /// transaction.
    pub cumulative_gas_per_block: BigUint,
}

/// Cheat instructions that can be sent to the [`Environment`] via the
/// [`Socket`].
#[derive(Debug, Clone)]
pub enum CheatInstruction {
    /// Declares a new contract.
    DeclareContract {
        /// The class hash of the contract to declare.
        // class_hash: Felt,
        /// The compiled Sierra JSON of the contract.
        sierra_json: String,
    },
    /// Creates a new account.
    CreateAccount {
        /// The public key for the new account.
        signing_key: SigningKey,
        /// The class hash for the new account.
        class_hash: Felt,
        /// The prefunded balance for the new account.
        prefunded_balance: BigUint,
    },
    /// Creates a new block.
    CreateBlock,
    /// Sends an L1 message.
    L1Message {
        /// The L1 handler transaction.
        l1_handler_transaction: L1HandlerTransaction,
    },
    /// Mints tokens in respective ERC20 contract for an account.
    TopUpBalance {
        /// The address to top up.
        receiver: Felt,
        /// The amount to add.
        amount: BigUint,
        /// The token symbol or identifier.
        token: TokenId, // need to create classifier
    },
    /// Fetches token balance for an account.
    GetBalance {
        /// The address to check.
        address: Felt,
        /// The token symbol or identifier.
        token: TokenId, // need to create classifier
    },
    /// Starts impersonation. Skips transaction validation if sent on behalf of
    /// an account with impersonated address.
    Impersonate {
        /// The address to impersonate.
        address: Felt,
    },
    /// Stops impersonation.
    StopImpersonating {
        /// The address to stop impersonating.
        address: Felt,
    },
    /// Sets a value in the storage slot of a contract.
    SetStorageAt {
        /// The address of the contract.
        address: Felt,
        /// The key of the storage slot.
        key: Felt,
        /// The value to set in the storage slot.
        value: Felt,
    },
    /// Sets the gas modification for the next block. (And created a block if
    /// needed)
    SetNextBlockGas {
        /// The gas modification request.
        gas_modification: GasModificationRequest,
    },
    /// Extracts the deployed contract address from a deployment transaction.
    GetDeployedContractAddress {
        /// The transaction hash of the deployment.
        tx_hash: Felt,
    },

    /// Gets a block with transactions by its identifier
    /// from the forked blockchain.
    GetBlockWithTxs {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
    },
    /// Fetches block with transactions by its identifier
    /// and adds them on top of pending block in DevNet.
    ReplayBlockWithTxs {
        /// The identifier of the block to retrieve.
        block_id: core_types::BlockId,
        /// Checks if transaction has events matching filters and applies it if
        /// so, otherwise ignores.
        has_events: Option<Vec<EventFilter>>,
        /// Set to true to recalculate the nonce for the transactions
        override_nonce: bool,
    },

    /// Fetches all events from block in one go. Only scans local blocks
    GetAllEvents {
        /// Block id of a the block to fetch events from. If `None`, fetches all
        /// events since fork start.
        from_block: Option<core_types::BlockId>,
        /// Block id to fetch events till. If `None`, fetches all events from
        /// from_block
        to_block: Option<core_types::BlockId>,
        /// The address that emitted events to filter events by. If `None`,
        /// fetches all events.
        address: Option<Felt>,
        /// The keys to filter events by. If `None`, fetches all events.
        keys: Option<Vec<Vec<Felt>>>,
    },
}

/// Return values of applying cheatcodes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CheatcodesOutcome {
    /// Returns the class hash of the declared contract.
    DeclareContract(Felt),
    /// Returns the contract address of the created account.
    CreateAccount(Felt),
    /// Indicates a block was created. Returns latest block hash.
    CreateBlock(Felt),
    /// Returns the tx_hash of L1 message transaction.
    L1Message(Felt),
    /// Indicates a balance was added.
    TopUpBalance,
    /// Returns amount of token owned.
    GetBalance(BigUint),
    /// Indicates the address was impersonated.
    Impersonate,
    /// Indicates the impersonation was stopped.
    StopImpersonating,
    /// Indicates the storage slot was set.
    SetStorageAt,
    /// Indicates the next block gas was set returning the gas modification
    /// values.
    SetNextBlockGas(GasModification),
    /// Returns the address of the deployed contract.
    GetDeployedContractAddress(Felt),
    /// Returns block with transactions.
    GetBlockWithTxs(Box<core_types::MaybePendingBlockWithTxs>),
    /// Returns numbers of transactions from the origin block that were (added,
    /// ignored, failed)
    ReplayBlockWithTxs(usize, usize, usize),

    /// Returns all events matching the filter.
    GetAllEvents(Vec<core_types::EmittedEvent>),
}
