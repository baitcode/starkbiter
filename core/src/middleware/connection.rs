//! Messengers/connections to the underlying EVM in the environment.
use std::{pin::Pin, sync::Weak};

use async_trait;
use futures::Stream;
use starknet::providers::{Provider, ProviderError};
use starknet_core::types::{self as core_types};
use starknet_devnet_types::num_bigint::BigUint;
use tokio::sync::broadcast;

use super::*;
use crate::{
    environment::{instruction::*, InstructionSender, OutcomeReceiver, OutcomeSender},
    tokens::TokenId,
};

/// Represents a connection to the EVM contained in the corresponding
/// [`Environment`].
#[derive(Debug, Clone)]
pub struct Connection {
    /// Used to send calls and transactions to the [`Environment`] to be
    /// executed by `revm`.
    pub(crate) instruction_sender: Weak<InstructionSender>,

    /// Used to send results back to a client that made a call/transaction with
    /// the [`Environment`]. This [`ResultSender`] is passed along with a
    /// call/transaction so the [`Environment`] can reply back with the
    /// [`ExecutionResult`].
    pub(crate) outcome_sender: OutcomeSender,

    /// Used to receive the [`ExecutionResult`] from the [`Environment`] upon
    /// call/transact.
    pub(crate) outcome_receiver: OutcomeReceiver,

    pub(crate) event_sender: broadcast::Sender<Vec<EmittedEvent>>,
}

impl From<&Environment> for Connection {
    fn from(environment: &Environment) -> Self {
        let instruction_sender = &Arc::clone(&environment.socket.instruction_sender);
        let (outcome_sender, outcome_receiver) = crossbeam_channel::unbounded();
        Self {
            instruction_sender: Arc::downgrade(instruction_sender),
            outcome_sender,
            outcome_receiver,
            event_sender: environment.socket.event_broadcaster.clone(),
        }
    }
}

impl Connection {
    //! Sends an instruction to the [`Environment`] and waits for the outcome
    async fn send_instruction_recv_outcome(
        &self,
        to_send: Instruction,
    ) -> Result<Outcome, ProviderError> {
        self.instruction_sender
            .upgrade() // TODO: WHY?!
            .ok_or(ProviderError::RateLimited)?
            .send((to_send, self.outcome_sender.clone()))
            .map_err(|_| ProviderError::RateLimited)?;

        let res = self
            .outcome_receiver
            .recv()
            // TODO: fix errors
            .map_err(|_| ProviderError::RateLimited)?
            .map_err(|e| {
                trace!("Error from environment outcome. It'll be dropped: {:?}", e);
                ProviderError::RateLimited
            })?;

        Ok(res)
    }

    /// Subscribes to a stream of emitted event vectors. Vectors are produced
    /// the moment new block is created. Events are filtered and only events
    /// of type `T` are returned.
    ///
    /// # Type Parameters
    /// * `T` - A type that can be constructed from a reference to
    ///   `EmittedEvent`.
    pub async fn subscribe_to<T>(&self) -> Pin<Box<dyn Stream<Item = Vec<T>> + Send + Sync>>
    where
        T: for<'a> TryFrom<&'a EmittedEvent> + Send + Sync,
    {
        let rx: broadcast::Receiver<Vec<EmittedEvent>> = self.event_sender.subscribe();

        let stream = futures::stream::unfold(rx, |mut rx| async {
            loop {
                match rx.recv().await {
                    Ok(val) => {
                        tracing::trace!("Received events: {:?}", val);

                        let res = val
                            .iter()
                            .map(|el| T::try_from(el))
                            .filter(|res| res.is_ok())
                            // TODO: fix unsafe
                            .map(|res| unsafe { res.unwrap_unchecked() })
                            .collect();

                        return Some((res, rx));
                    }
                    Err(_) => continue,
                }
            }
        });

        Box::pin(stream)
    }
}

#[async_trait::async_trait]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl CheatingProvider for Connection {
    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::GetDeployedContractAddress {
            tx_hash: tx_hash.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::GetDeployedContractAddress(address)) = res {
            Ok(address)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn create_block(&self) -> Result<Felt, ProviderError> {
        let to_send = Instruction::Cheat(CheatInstruction::CreateBlock {});

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::CreateBlock(block_hash)) = res {
            Ok(block_hash)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn create_account<V, F, I>(
        &self,
        signing_key: V,
        class_hash: F,
        prefunded_balance: I,
    ) -> Result<Felt, ProviderError>
    where
        V: Into<SigningKey> + Send + Sync, // change for Into
        F: Into<Felt> + Send + Sync,
        I: Into<BigUint> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::CreateAccount {
            signing_key: signing_key.into(),
            class_hash: class_hash.into(),
            prefunded_balance: prefunded_balance.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::CreateAccount(address)) = res {
            Ok(address)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn top_up_balance<C, B, T>(
        &self,
        receiver: C,
        amount: B,
        token: T,
    ) -> Result<(), ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        B: Into<BigUint> + Send + Sync,
        T: Into<TokenId> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::TopUpBalance {
            receiver: receiver.into(),
            amount: amount.into(),
            token: token.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::TopUpBalance) = res {
            Ok(())
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn get_balance<C, T>(&self, receiver: C, token: T) -> Result<BigUint, ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        T: Into<TokenId> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::GetBalance {
            address: receiver.into(),
            token: token.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::GetBalance(value)) = res {
            Ok(value)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::Impersonate {
            address: *address.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::Impersonate) = res {
            Ok(())
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::StopImpersonating {
            address: *address.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::StopImpersonating) = res {
            Ok(())
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn set_storage_at<C, K, V>(
        &self,
        address: C,
        key: K,
        value: V,
    ) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
        K: AsRef<Felt> + Send + Sync,
        V: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::SetStorageAt {
            address: *address.as_ref(),
            key: *key.as_ref(),
            value: *value.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::SetStorageAt) = res {
            Ok(())
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::DeclareContract {
            sierra_json: sierra_json.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::DeclareContract(class_hash)) = res {
            Ok(class_hash)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn set_next_block_gas<G>(
        &self,
        gas_modification_request: G,
    ) -> Result<GasModification, ProviderError>
    where
        G: Into<GasModificationRequest> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::SetNextBlockGas {
            gas_modification: gas_modification_request.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::SetNextBlockGas(gas)) = res {
            Ok(gas)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn get_block_with_txs_from_fork<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::GetBlockWithTxs {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::GetBlockWithTxs(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn add_l1_handler_transaction<T>(&self, tx: T) -> Result<Felt, ProviderError>
    where
        T: Into<core_types::L1HandlerTransaction> + Send + Sync,
    {
        let handler: &core_types::L1HandlerTransaction = &tx.into();

        // TODO(baitcode): Hmmm to many conversions here, need to rethink this
        let to_send = Instruction::Cheat(CheatInstruction::L1Message {
            l1_handler_transaction: handler.into(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::L1Message(tx_hash)) = res {
            Ok(tx_hash)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn replay_block_with_txs<B, F>(
        &self,
        block_id: B,
        filters: F,
        override_nonce: bool,
    ) -> Result<(usize, usize, usize), ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        F: Into<Option<Vec<EventFilter>>> + Send + Sync,
    {
        let to_send = Instruction::Cheat(CheatInstruction::ReplayBlockWithTxs {
            block_id: *block_id.as_ref(),
            has_events: filters.into(),
            override_nonce,
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::ReplayBlockWithTxs(added, ignored, failed)) = res {
            Ok((added, ignored, failed))
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn get_all_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<Felt>,
        keys: Option<Vec<Vec<Felt>>>,
    ) -> Result<Vec<core_types::EmittedEvent>, ProviderError> {
        let to_send = Instruction::Cheat(CheatInstruction::GetAllEvents {
            from_block,
            to_block,
            address,
            keys,
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Cheat(CheatcodesOutcome::GetAllEvents(events)) = res {
            Ok(events)
        } else {
            Err(ProviderError::RateLimited)
        }
    }
}

#[async_trait::async_trait]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl Provider for Connection {
    async fn spec_version(&self) -> Result<String, ProviderError> {
        return Ok("devnet".to_string());
    }

    async fn get_block_with_tx_hashes<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxHashes, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetBlockWithTxHashes {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetBlockWithTxHashes(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    async fn get_block_with_txs<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetBlockWithTxs {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetBlockWithTxs(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets block information with full transactions and receipts given the
    /// block id.
    async fn get_block_with_receipts<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithReceipts, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetBlockWithReceipts {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetBlockWithReceipts(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the information about the result of executing the requested block.
    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetStateUpdate {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetStateUpdate(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the value of the storage at the given address and key.
    async fn get_storage_at<A, K, B>(
        &self,
        contract_address: A,
        key: K,
        block_id: B,
    ) -> Result<Felt, ProviderError>
    where
        A: AsRef<Felt> + Send + Sync,
        K: AsRef<Felt> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetStorageAt {
            contract_address: *contract_address.as_ref(),
            key: *key.as_ref(),
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetStorageAt(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Given an l1 tx hash, returns the associated l1_handler tx hashes and
    /// statuses for all L1 -> L2 messages sent by the l1 transaction,
    /// ordered by the l1 tx sending order
    async fn get_messages_status(
        &self,
        transaction_hash: core_types::Hash256,
    ) -> Result<Vec<core_types::MessageWithStatus>, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::GetMessagesStatus { transaction_hash });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetMessagesStatus(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the transaction status (possibly reflecting that the tx is still in
    /// the mempool, or dropped from it).
    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetTransactionStatus {
            transaction_hash: core_types::Hash256::from_felt(transaction_hash.as_ref()),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetTransactionStatus(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the details and status of a submitted transaction.
    async fn get_transaction_by_hash<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetTransactionByHash {
            transaction_hash: core_types::Hash256::from_felt(transaction_hash.as_ref()),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetTransactionByHash(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the details of a transaction by a given block id and index.
    async fn get_transaction_by_block_id_and_index<B>(
        &self,
        block_id: B,
        index: u64,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetTransactionByBlockIdAndIndex {
            block_id: *block_id.as_ref(),
            index,
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetTransactionByBlockIdAndIndex(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the details of a transaction by a given block number and index.
    async fn get_transaction_receipt<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionReceiptWithBlockInfo, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetTransactionReceipt {
            transaction_hash: core_types::Hash256::from_felt(transaction_hash.as_ref()),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetTransactionReceipt(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the contract class definition in the given block associated with
    /// the given hash.
    async fn get_class<B, H>(
        &self,
        block_id: B,
        class_hash: H,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        H: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetClass {
            block_id: *block_id.as_ref(),
            class_hash: *class_hash.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetClass(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the contract class hash in the given block for the contract
    /// deployed at the given address.
    async fn get_class_hash_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetClassHashAt {
            block_id: *block_id.as_ref(),
            contract_address: *contract_address.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetClassHashAt(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the contract class definition in the given block at the given
    /// address.
    async fn get_class_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetClassAt {
            block_id: *block_id.as_ref(),
            contract_address: *contract_address.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetClassAt(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the number of transactions in a block given a block id.
    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetBlockTransactionCount {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetBlockTransactionCount(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Calls a starknet function without creating a Starknet transaction.
    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<core_types::FunctionCall> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::Call {
            request: request.as_ref().clone(),
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::Call(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Estimates the fee for a given Starknet transaction.
    async fn estimate_fee<R, S, B>(
        &self,
        request: R,
        simulation_flags: S,
        block_id: B,
    ) -> Result<Vec<core_types::FeeEstimate>, ProviderError>
    where
        R: AsRef<[core_types::BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[core_types::SimulationFlagForEstimateFee]> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let req = request.as_ref();

        if req.len() > 1 {
            unimplemented!("Estimate fee for multiple transactions is not implemented yet");
        }

        let flags: &[core_types::SimulationFlagForEstimateFee] = simulation_flags.as_ref();

        let to_send = Instruction::Node(NodeInstruction::EstimateFee {
            request: req[0].clone(),
            simulate_flags: flags.to_vec(),
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::EstimateFee(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Estimates the fee for sending an L1-to-L2 message.
    async fn estimate_message_fee<M, B>(
        &self,
        message: M,
        block_id: B,
    ) -> Result<core_types::FeeEstimate, ProviderError>
    where
        M: AsRef<core_types::MsgFromL1> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::EstimateMessageFee {
            message: message.as_ref().clone(),
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::EstimateMessageFee(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the most recent accepted block number.
    async fn block_number(&self) -> Result<u64, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::BlockNumber);

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::BlockNumber(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the most recent accepted block hash and number.
    async fn block_hash_and_number(&self) -> Result<core_types::BlockHashAndNumber, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::BlockHashAndNumber);

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::BlockHashAndNumber(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Returns the currently configured Starknet chain id.
    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::ChainId);

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::ChainId(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Returns an object about the sync status, or false if the node is not
    /// syncing.
    async fn syncing(&self) -> Result<core_types::SyncStatusType, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::Syncing);

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::Syncing(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Returns all events matching the given filter.
    async fn get_events(
        &self,
        filter: core_types::EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<core_types::EventsPage, ProviderError> {
        let to_send = Instruction::Node(NodeInstruction::GetEvents {
            filter: core_types::EventFilter {
                from_block: filter.from_block,
                to_block: filter.to_block,
                address: filter.address,
                keys: filter.keys,
            },
            continuation_token,
            chunk_size: Option::Some(chunk_size),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetEvents(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Gets the nonce associated with the given address in the given block.
    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::GetNonce {
            block_id: *block_id.as_ref(),
            contract_address: *contract_address.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::GetNonce(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Get merkle paths in one of the state tries: global state, classes,
    /// individual contract. A single request can query for any mix of the
    /// three types of storage proofs (classes, contracts, and storage).
    async fn get_storage_proof<B, H, A, K>(
        &self,
        _block_id: B,
        _class_hashes: H,
        _contract_addresses: A,
        _contracts_storage_keys: K,
    ) -> Result<core_types::StorageProof, ProviderError>
    where
        B: AsRef<core_types::ConfirmedBlockId> + Send + Sync,
        H: AsRef<[Felt]> + Send + Sync,
        A: AsRef<[Felt]> + Send + Sync,
        K: AsRef<[core_types::ContractStorageKeys]> + Send + Sync,
    {
        unimplemented!("get_storage_proof is not supported");
    }

    /// Submits a new transaction to be added to the chain.
    async fn add_invoke_transaction<I>(
        &self,
        invoke_transaction: I,
    ) -> Result<core_types::InvokeTransactionResult, ProviderError>
    where
        I: AsRef<core_types::BroadcastedInvokeTransaction> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::AddInvokeTransaction {
            transaction: invoke_transaction.as_ref().clone(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::AddInvokeTransaction(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Submits a new transaction to be added to the chain.
    async fn add_declare_transaction<D>(
        &self,
        declare_transaction: D,
    ) -> Result<core_types::DeclareTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeclareTransaction> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::AddDeclareTransaction {
            transaction: declare_transaction.as_ref().clone(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::AddDeclareTransaction(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Submits a new deploy account transaction.
    async fn add_deploy_account_transaction<D>(
        &self,
        deploy_account_transaction: D,
    ) -> Result<core_types::DeployAccountTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeployAccountTransaction> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::AddDeployAccountTransaction {
            transaction: deploy_account_transaction.as_ref().clone(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::AddDeployAccountTransaction(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// For a given executed transaction, returns the trace of its execution,
    /// including internal calls.
    async fn trace_transaction<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionTrace, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::TraceTransaction {
            transaction_hash: *transaction_hash.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::TraceTransaction(res)) = res {
            Ok(*res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Simulates a given sequence of transactions on the requested state, and
    /// generate the execution traces. Note that some of the transactions
    /// may revert, in which case no error is thrown, but revert details can
    /// be seen on the returned trace object.
    ///
    /// Note that some of the transactions may revert, this will be reflected by
    /// the `revert_error` property in the trace. Other types of failures
    /// (e.g. unexpected error or failure in the validation phase) will
    /// result in `TRANSACTION_EXECUTION_ERROR`.
    async fn simulate_transactions<B, T, S>(
        &self,
        block_id: B,
        transactions: T,
        simulation_flags: S,
    ) -> Result<Vec<core_types::SimulatedTransaction>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        T: AsRef<[core_types::BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[core_types::SimulationFlag]> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::SimulateTransactions {
            block_id: *block_id.as_ref(),
            transactions: transactions.as_ref().to_vec(),
            simulation_flags: simulation_flags.as_ref().to_vec(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::SimulateTransactions(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Retrieves traces for all transactions in the given block.
    async fn trace_block_transactions<B>(
        &self,
        block_id: B,
    ) -> Result<Vec<core_types::TransactionTraceWithHash>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        let to_send = Instruction::Node(NodeInstruction::TraceBlockTransactions {
            block_id: *block_id.as_ref(),
        });

        let res = self.send_instruction_recv_outcome(to_send).await?;

        if let Outcome::Node(NodeOutcome::TraceBlockTransactions(res)) = res {
            Ok(res)
        } else {
            Err(ProviderError::RateLimited)
        }
    }

    /// Sends multiple requests in parallel. The function call fails if any of
    /// the requests fails. Implementations must guarantee that responses
    /// follow the exact order as the requests.
    async fn batch_requests<R>(
        &self,
        _requests: R,
    ) -> Result<Vec<ProviderResponseData>, ProviderError>
    where
        R: AsRef<[ProviderRequestData]> + Send + Sync,
    {
        unimplemented!("Batch requests are not supported in the current implementation");
    }
}
