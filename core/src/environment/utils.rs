use super::instruction::TxExecutionResult;
use crate::errors::ArbiterCoreError;

use starknet::core::types::{Felt, TransactionExecutionStatus};

use starknet_core::utils::get_storage_var_address;
use starknet_devnet_core::{
    constants::ISRC6_ID_HEX, error::Error as DevnetError, starknet::Starknet, state::StarknetState,
    state::StateReader,
};

use starknet_devnet_types::{
    felt::{felt_from_prefixed_hex, join_felts},
    num_bigint::BigUint,
    rpc::{
        felt::split_biguint,
        transactions::{BroadcastedTransaction, TransactionTrace},
    },
    starknet_api::{core::ContractAddress, state::StorageKey},
};

pub fn execute_transaction(
    starknet: &mut Starknet,
    tx: BroadcastedTransaction,
) -> Result<TxExecutionResult, ArbiterCoreError> {
    let transaction_hash = match tx {
        BroadcastedTransaction::Invoke(tx) => starknet.add_invoke_transaction(tx),
        BroadcastedTransaction::DeployAccount(tx) => starknet
            .add_deploy_account_transaction(tx)
            .map(|(hash, _)| hash),
        BroadcastedTransaction::Declare(tx) => {
            starknet.add_declare_transaction(tx).map(|(hash, _)| hash)
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
            let reason = trasaction_status
                .failure_reason
                .or(Some("unknown".to_string()))
                .unwrap();

            TxExecutionResult::Revert(reason, tx_receipt)
        }
    };
    Ok(result)
}

// impl From<starknet_devnet_types::contract_address::ContractAddress> for ContractAddress {

//     fn from(address: starknet_api::ContractAddress) -> Self {
//         ContractAddress::new(address.0).unwrap()
//     }

// }

pub fn mint_tokens_in_erc20_contract(
    state: &mut StarknetState,
    contract_address: Felt,
    recipient: Felt,
    amount: BigUint,
) -> Result<(), ArbiterCoreError> {
    let contract_address = ContractAddress::try_from(contract_address)
        .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

    fn read_biguint(
        state: &StarknetState,
        address: ContractAddress,
        low_key: Felt,
    ) -> Result<BigUint, ArbiterCoreError> {
        let low_storage_key = StorageKey::try_from(low_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_storage_key = StorageKey::try_from(low_key + Felt::ONE)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let low_val = state
            .get_storage_at(address, low_storage_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        let high_val = state
            .get_storage_at(address, high_storage_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        return Ok(join_felts(&high_val, &low_val));
    }

    fn write_biguint(
        state: &mut StarknetState,
        address: ContractAddress,
        low_key: Felt,
        value: BigUint,
    ) -> Result<(), ArbiterCoreError> {
        let low_storage_key = StorageKey::try_from(low_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_storage_key = StorageKey::try_from(low_key + Felt::ONE)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let (high_val, low_val) = split_biguint(value);

        state
            .state
            .state
            .set_storage_at(address, low_storage_key, low_val)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        state
            .state
            .state
            .set_storage_at(address, high_storage_key, high_val)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        Ok(())
    }

    let recepient_balance_key = get_storage_var_address("ERC20_balances", &[recipient])
        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

    let recepient_balance = read_biguint(&state, contract_address, recepient_balance_key)?;

    let total_supply_key = get_storage_var_address("ERC20_total_supply", &[])
        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

    let total_supply = read_biguint(&state, contract_address, total_supply_key)?;

    write_biguint(
        state,
        contract_address,
        recepient_balance_key,
        recepient_balance + amount.clone(),
    )?;

    write_biguint(
        state,
        contract_address,
        total_supply_key,
        total_supply + amount.clone(),
    )?;

    Ok(())
}

pub fn simulate_constructor(
    state: &mut StarknetState,
    contract_address: ContractAddress,
    public_key: Felt,
) -> Result<(), ArbiterCoreError> {
    let core_address = contract_address.try_into().unwrap();

    let interface_selector = felt_from_prefixed_hex(ISRC6_ID_HEX)
        .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

    let interface_storage_var: StorageKey =
        get_storage_var_address("SRC5_supported_interfaces", &[interface_selector])
            .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?
            .try_into()
            .unwrap(); // should be safe as we control the selector

    state
        .state
        .state
        .set_storage_at(core_address, interface_storage_var, Felt::ONE)
        .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

    let public_key_storage_var: StorageKey = get_storage_var_address("Account_public_key", &[])
        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?
        .try_into() // should be safe as we control the selector
        .unwrap();

    state
        .state
        .state
        .set_storage_at(core_address, public_key_storage_var, public_key)
        .map_err(|e| ArbiterCoreError::DevnetError(e.into()))?;

    Ok(())
}
