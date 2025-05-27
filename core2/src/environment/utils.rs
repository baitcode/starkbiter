use super::instruction::TxExecutionResult;
use crate::errors::ArbiterCoreError;
use starknet::core::{
    types::{BroadcastedTransaction, Felt, TransactionExecutionStatus},
    utils::get_storage_var_address,
};
use starknet_devnet_core::{starknet::Starknet, state::StarknetState};
use starknet_devnet_types::{
    contract_address::ContractAddress,
    felt::{join_felts, split_biguint},
    num_bigint::BigUint,
    patricia_key::StorageKey,
    rpc::transactions::TransactionTrace,
};

pub fn execute_transaction(
    starknet: &mut Starknet,
    zzzz: BroadcastedTransaction,
) -> Result<TxExecutionResult, ArbiterCoreError> {
    let transaction_hash = match zzzz {
        BroadcastedTransaction::Invoke(zzzz) => starknet.add_invoke_transaction(zzzz),
        BroadcastedTransaction::DeployAccount(_) => starknet
            .add_deploy_account_transaction(zzzz)
            .map(|(hash, _)| hash),
        BroadcastedTransaction::Declare(_) => {
            starknet.add_declare_transaction(zzzz).map(|(hash, _)| hash)
        }
    }
    .map_err(|e| ArbiterCoreError::DevnetError(e))?;

    let trasaction_status = starknet
        .get_transaction_execution_and_finality_status(transaction_hash)
        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

    let transaction_trace = starknet
        .get_transaction_trace_by_hash(transaction_hash)
        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

    // TODO: extract execution logs here.
    let gas_used = match transaction_trace {
        TransactionTrace::Invoke(trace) => trace.execution_resources.l2_gas,
        TransactionTrace::Declare(trace) => trace.execution_resources.l2_gas,
        TransactionTrace::DeployAccount(trace) => trace.execution_resources.l2_gas,
        TransactionTrace::L1Handler(trace) => unreachable!(),
    };

    let tx_receipt = starknet
        .get_transaction_receipt_by_hash(&transaction_hash)
        .map_err(|e| ArbiterCoreError::DevnetError(e))?;

    let result = match trasaction_status.execution_status {
        TransactionExecutionStatus::Succeeded => {
            TxExecutionResult::Success(transaction_hash, tx_receipt)
        }
        TransactionExecutionStatus::Reverted => {
            TxExecutionResult::Revert(trasaction_status.failure_reason.or("unknown"), tx_receipt)
        }
    };
    Ok(result)
}

pub fn mint_tokens_in_erc20_contract(
    starknet: &Starknet,
    contract_address: &ContractAddress,
    recipient: &Felt,
    amount: u64,
) -> Result<TxExecutionResult, ArbiterCoreError> {
    // pass
    let state = starknet.get_state();

    fn read_biguint(
        state: &StarknetState,
        address: &ContractAddress,
        low_key: &StorageKey,
    ) -> Result<BigUint, ArbiterCoreError> {
        let high_key = low_key.next_storage_key();
        return Ok(join_felts(
            state.get_storage_at(address, high_key),
            state.get_storage_at(address, low_key),
        ));
    }

    fn write_biguint(
        state: &mut StarknetState,
        address: &ContractAddress,
        low_key: &StorageKey,
        value: BigUint,
    ) -> Result<(), ArbiterCoreError> {
        let high_key = low_key.next_storage_key();
        let (high, low) = split_biguint(value);
        state.set_storage_at(address, low_key, low)?;
        state.set_storage_at(address, high_key, high)?;
        Ok(())
    }

    let recepient_balance_key =
        get_storage_var_address("ERC20_balances", &[Felt::from(recipient)])?.try_into()?;
    let recepient_balance = read_biguint(&state, contract_address, recepient_balance_key)?;

    let total_supply_key = get_storage_var_address("ERC20_total_supply", &[])?.try_into()?;
    let total_supply = read_biguint(&state, contract_address, total_supply_key);

    write_biguint(
        &mut state,
        contract_address,
        recepient_balance_key,
        recepient_balance + amount,
    )?;

    write_biguint(
        &mut state,
        contract_address,
        total_supply_key,
        recepient_balance + amount,
    )?;
}
