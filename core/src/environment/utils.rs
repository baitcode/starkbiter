use crate::errors::ArbiterCoreError;

use starknet::core::types::Felt;

use starknet_core::utils::get_storage_var_address;
use starknet_devnet_core::{
    constants::ISRC6_ID_HEX,
    error::Error as DevnetError,
    state::{StarknetState, StateReader},
};

use starknet_devnet_types::{
    felt::{felt_from_prefixed_hex, join_felts},
    num_bigint::BigUint,
    rpc::felt::split_biguint,
    starknet_api::{core::ContractAddress, state::StorageKey},
};
use tracing::trace;

pub fn mint_tokens_in_erc20_contract(
    state: &mut StarknetState,
    contract_address: Felt,
    recipient: Felt,
    amount: BigUint,
) -> Result<(), ArbiterCoreError> {
    let contract_address = ContractAddress::try_from(contract_address)
        .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

    fn read_biguint(
        msg: String,
        state: &StarknetState,
        address: ContractAddress,
        low_key: Felt,
    ) -> Result<BigUint, ArbiterCoreError> {
        let low_key = StorageKey::try_from(low_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_key = low_key
            .next_storage_key()
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let low_val = state
            .get_storage_at(address, low_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        let high_val = state
            .get_storage_at(address, high_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        return Ok(join_felts(&high_val, &low_val));
    }

    fn write_biguint(
        msg: String,
        state: &mut StarknetState,
        address: ContractAddress,
        low_key: Felt,
        value: BigUint,
    ) -> Result<(), ArbiterCoreError> {
        let low_key = StorageKey::try_from(low_key)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let high_key = low_key
            .next_storage_key()
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::StarknetApiError(e)))?;

        let (high_val, low_val) = split_biguint(value);

        state
            .state
            .state
            .set_storage_at(address, low_key, low_val)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        state
            .state
            .state
            .set_storage_at(address, high_key, high_val)
            .map_err(|e| ArbiterCoreError::DevnetError(DevnetError::BlockifierStateError(e)))?;

        Ok(())
    }

    let recepient_balance_key = get_storage_var_address("ERC20_balances", &[recipient])
        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

    let recepient_balance = read_biguint(
        "recepient_balance".to_string(),
        &state,
        contract_address,
        recepient_balance_key,
    )?;

    let total_supply_key = get_storage_var_address("ERC20_total_supply", &[])
        .map_err(|e| ArbiterCoreError::InternalError(e.to_string()))?;

    let total_supply = read_biguint(
        "total_supply".to_string(),
        &state,
        contract_address,
        total_supply_key,
    )?;

    write_biguint(
        "recepient_balance".to_string(),
        state,
        contract_address,
        recepient_balance_key,
        recepient_balance + amount.clone(),
    )?;

    write_biguint(
        "total_supply".to_string(),
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
