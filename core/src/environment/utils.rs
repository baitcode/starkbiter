use starknet::core::types::Felt;
use starknet_core::utils::get_storage_var_address;
use starknet_devnet_core::{
    error::Error as DevnetError,
    state::{StarknetState, StateReader},
};
use starknet_devnet_types::{
    felt::join_felts,
    num_bigint::BigUint,
    rpc::felt::split_biguint,
    starknet_api::{core::ContractAddress, state::StorageKey},
};

use crate::errors::StarkbiterCoreError;

/// This method utilizes direct access to Starknet state to mint tokens in an
/// ERC20 contract.
pub fn mint_tokens_in_erc20_contract(
    state: &mut StarknetState,
    contract_address: Felt,
    recipient: Felt,
    amount: BigUint,
) -> Result<(), Box<StarkbiterCoreError>> {
    let contract_address = ContractAddress::try_from(contract_address)
        .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

    fn read_biguint(
        state: &StarknetState,
        address: ContractAddress,
        low_key: Felt,
    ) -> Result<BigUint, StarkbiterCoreError> {
        let low_key = StorageKey::try_from(low_key)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_key = low_key
            .next_storage_key()
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let low_val = state
            .get_storage_at(address, low_key)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        let high_val = state
            .get_storage_at(address, high_key)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        Ok(join_felts(&high_val, &low_val))
    }

    fn write_biguint(
        state: &mut StarknetState,
        address: ContractAddress,
        low_key: Felt,
        value: BigUint,
    ) -> Result<(), StarkbiterCoreError> {
        let low_key = StorageKey::try_from(low_key)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_key = low_key
            .next_storage_key()
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let (high_val, low_val) = split_biguint(value);

        state
            .state
            .state
            .set_storage_at(address, low_key, low_val)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        state
            .state
            .state
            .set_storage_at(address, high_key, high_val)
            .map_err(|e| StarkbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        Ok(())
    }

    let recepient_balance_key = get_storage_var_address("ERC20_balances", &[recipient])
        .map_err(|e| StarkbiterCoreError::InternalError(e.to_string()))?;

    let recepient_balance = read_biguint(state, contract_address, recepient_balance_key)?;

    let total_supply_key = get_storage_var_address("ERC20_total_supply", &[])
        .map_err(|e| StarkbiterCoreError::InternalError(e.to_string()))?;

    let total_supply = read_biguint(state, contract_address, total_supply_key)?;

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
