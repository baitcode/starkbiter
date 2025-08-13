use std::sync::Arc;

/// SQLiteStateReader
/// This module provides a reader for the Starknet state stored in a SQLite
/// database. It is compatible with starknet-devnet defaulter implementation,
/// and allows to replace node api http requests for state storage data with
/// SQLite queries for Pathfinder formatted sqlite database.
use blockifier::{
    execution::contract_class::RunnableCompiledClass,
    state::{errors::StateError, state_api::StateResult},
};
use r2d2_sqlite::{
    rusqlite::{params, OptionalExtension},
    SqliteConnectionManager,
};
use starknet_devnet_core::starknet::defaulter::{OriginReader, StarknetDefaulter};
use url::Url;

#[derive(Debug, Clone)]
/// Represents a reader for the Starknet state stored in a SQLite database.
/// This reader allows querying the state at a specific block number.
/// It implements the `OriginReader` trait, which provides methods to read
/// storage, nonce, compiled classes, and class hashes from the state.
pub struct SQLiteStateReader {
    block_number: u64,
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl SQLiteStateReader {
    /// Factory function to create a new SQLiteStateReader. Complying with the
    /// OriginReaderFactory type from starknet-devnet fork.
    pub fn new_sqlite_state_reader(url: Url, block_number: u64) -> StarknetDefaulter {
        let manager = SqliteConnectionManager::file(url.path());
        let pool = r2d2::Pool::new(manager).unwrap();
        StarknetDefaulter::new_with_reader(Arc::new(SQLiteStateReader { block_number, pool }))
    }
}

impl OriginReader for SQLiteStateReader {
    fn get_storage_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
        key: starknet_api::state::StorageKey,
    ) -> StateResult<starknet_core::types::Felt> {
        let connection = self.pool.get().unwrap();
        let result = connection.query_one::<Vec<u8>, _, _>(
            r"
                SELECT storage_value
                FROM storage_updates
                JOIN contract_addresses ON contract_addresses.id = storage_updates.contract_address_id
                JOIN storage_addresses ON storage_addresses.id = storage_updates.storage_address_id
                WHERE contract_address = ?
                  AND storage_address = ?
                  AND block_number <= ?
                ORDER BY block_number DESC LIMIT 1
            ",
            params![contract_address.to_bytes_be(), key.to_bytes_be(), self.block_number],
            |row| row.get(0)
        )
        .optional()
        .map_err(|err| {
            StateError::StateReadError(format!("Failed to read storage. {}", err))
        })?;

        Ok(starknet_core::types::Felt::from_bytes_be_slice(
            result.unwrap_or_default().as_slice(),
        ))
    }

    fn get_nonce_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> StateResult<starknet_api::core::Nonce> {
        let connection = self.pool.get().unwrap();
        let result = connection
            .query_one::<Vec<u8>, _, _>(
                r"
                SELECT nonce
                FROM nonce_updates
                JOIN contract_addresses ON contract_addresses.id = nonce_updates.contract_address_id
                WHERE contract_address = ?
                  AND block_number <= ?
                ORDER BY block_number DESC LIMIT 1
                ",
                params![contract_address.to_bytes_be(), self.block_number],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| StateError::StateReadError(format!("Failed to read nonce. {}", err)))?;
        let felt =
            starknet_core::types::Felt::from_bytes_be_slice(result.unwrap_or_default().as_slice());
        Ok(starknet_api::core::Nonce(felt)) // Placeholder implementation
    }

    fn get_compiled_class(
        &self,
        class_hash: starknet_api::core::ClassHash,
    ) -> StateResult<RunnableCompiledClass> {
        let connection = self.pool.get().unwrap();

        let casm_definition = connection
            .query_row::<Vec<u8>, _, _>(
                r"
                SELECT
                    casm_definitions.definition,
                    class_definitions.block_number
                FROM
                    casm_definitions
                    INNER JOIN class_definitions ON (
                        class_definitions.hash = casm_definitions.hash
                    )
                WHERE
                    casm_definitions.hash = ?
                    AND class_definitions.block_number <= ?",
                params![class_hash.to_bytes_be(), self.block_number],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| {
                StateError::StateReadError(format!("Failed to read storage. {}", err))
            })?;

        let class_definition = connection
            .query_row::<Vec<u8>, _, _>(
                r"
                SELECT
                    definition,
                    block_number
                FROM
                    class_definitions
                WHERE 1=1
                  AND hash = :hash
                  AND block_number <= :block_number
                ORDER BY block_number DESC
                LIMIT 1
                ",
                params![class_hash.to_bytes_be(), self.block_number],
                |row| {
                    let rf = row.get_ref(0)?;
                    let blob = rf.as_blob()?;
                    Ok(blob.to_vec())
                },
            )
            .map_err(|err| StateError::StateReadError(format!("Failed to create stmt. {}", err)))?;

        // If we have a CASM class_definition, we can return it
        let class_definition = zstd::decode_all(class_definition.as_slice()).map_err(|err| {
            StateError::StateReadError(format!("Decompressing compiled class definition. {}", err))
        })?;

        let class_definition_str = String::from_utf8(class_definition).map_err(|error| {
            StateError::StateReadError(format!("Class definition is not valid UTF-8: {error}"))
        })?;

        if let Some(casm_definition) = casm_definition {
            let json_repr: serde_json::Value = serde_json::from_str(&class_definition_str)
                .map_err(|err| {
                    StateError::StateReadError(format!(
                        "Decompressing compiled class definition. {}",
                        err
                    ))
                })?;

            let repr = json_repr.as_object().unwrap();
            let program = repr.get("sierra_program").unwrap().as_array().unwrap();
            let (version_components_raw, _) = program.split_first_chunk::<3>().unwrap();

            let version_components = version_components_raw.clone().map(|s| {
                u64::from_str_radix(s.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
            });

            let version = starknet_api::contract_class::SierraVersion::new(
                version_components[0],
                version_components[1],
                version_components[2],
            );

            // If we have a class definition, we can return it
            let casm_definition = zstd::decode_all(casm_definition.as_slice()).map_err(|err| {
                StateError::StateReadError(format!(
                    "Decompressing compiled class definition. {}",
                    err
                ))
            })?;

            let casm_definition = String::from_utf8(casm_definition).map_err(|error| {
                StateError::StateReadError(format!("CASM definition is not valid UTF-8: {error}"))
            })?;

            let casm_class =
                blockifier::execution::contract_class::CompiledClassV1::try_from_json_string(
                    &casm_definition,
                    version,
                )
                .map_err(StateError::ProgramError)?;

            Ok(RunnableCompiledClass::V1(casm_class))
        } else {
            let class =
                blockifier::execution::contract_class::CompiledClassV0::try_from_json_string(
                    &class_definition_str,
                )
                .map_err(StateError::ProgramError)?;

            Ok(RunnableCompiledClass::V0(class))
        }
    }

    fn get_class_hash_at(
        &self,
        contract_address: starknet_api::core::ContractAddress,
    ) -> StateResult<starknet_api::core::ClassHash> {
        let connection = self.pool.get().unwrap();
        let result = connection
            .query_one::<Vec<u8>, _, _>(
                r"
                SELECT
                    class_hash
                FROM contract_updates
                WHERE contract_address = ?
                  AND block_number <= ?
                ORDER BY block_number DESC
                LIMIT 1",
                params![contract_address.to_bytes_be(), self.block_number],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| {
                StateError::StateReadError(format!("Failed to read storage. {}", err))
            })?;
        let stark_hash =
            starknet_core::types::Felt::from_bytes_be_slice(result.unwrap_or_default().as_slice());
        Ok(starknet_api::core::ClassHash(stark_hash))
    }
}

#[cfg(test)]
mod tests {

    use starknet_api::abi::abi_utils::get_storage_var_address;
    use starknet_core::types::Felt;

    use super::*;

    #[test]
    fn test_sqlite_get_compiled_class_v1() {
        let dir = std::env::current_dir().unwrap();
        let db_path = format!("sqlite://{}/mainnet-trimmed.sqlite", dir.to_str().unwrap());

        let reader =
            SQLiteStateReader::new_sqlite_state_reader(Url::parse(&db_path).unwrap(), 1642947);

        let class_hash =
            starknet_api::core::ClassHash(starknet_core::types::Felt::from_hex_unchecked(
                "070D4F063FE6CA22667E3B451D9AAF298337153982D45DD91CFFD2AE7939C074",
            ));
        let res = reader.get_compiled_class(class_hash);

        match res {
            Ok(RunnableCompiledClass::V1(_)) => {}
            _ => {
                assert!(false, "Expected a V1 compiled class. Got: {:?}", res);
            }
        }
    }

    #[test]
    fn test_sqlite_get_compiled_class_v0() {
        let dir = std::env::current_dir().unwrap();
        let db_path = format!("sqlite://{}/mainnet-trimmed.sqlite", dir.to_str().unwrap());

        let reader =
            SQLiteStateReader::new_sqlite_state_reader(Url::parse(&db_path).unwrap(), 1642947);

        let class_hash =
            starknet_api::core::ClassHash(starknet_core::types::Felt::from_hex_unchecked(
                "03a140031fa515aab4e798a9d9265db749f9ea49d2ecb839a2efcc9e0f57b10e",
            ));
        let res = reader.get_compiled_class(class_hash);

        match res {
            Ok(RunnableCompiledClass::V0(_)) => {}
            _ => {
                assert!(false, "Expected a V1 compiled class. Got: {:?}", res);
            }
        }
    }

    #[test]
    fn test_sqlite_get_storage_at() {
        let dir = std::env::current_dir().unwrap();
        let db_path = format!("sqlite://{}/mainnet-trimmed.sqlite", dir.to_str().unwrap());

        let reader =
            SQLiteStateReader::new_sqlite_state_reader(Url::parse(&db_path).unwrap(), 1642947);

        let address = get_storage_var_address(
            "ERC20_balances",
            [
                starknet_core::types::Felt::from_hex_unchecked(
                    "0x049D36570D4e46f48e99674bd3fcc84644DdD6b96F7C741B1562B82f9e004dC7",
                ), // Stark Gate contract address
            ]
            .as_slice(),
        );

        // Ether: Stark Gate
        let contract_address = starknet_api::core::ContractAddress::try_from(
            starknet_core::types::Felt::from_hex_unchecked(
                "0x049D36570D4e46f48e99674bd3fcc84644DdD6b96F7C741B1562B82f9e004dC7",
            ),
        )
        .unwrap();

        println!("Address: {:?}", address);

        let res = reader.get_storage_at(contract_address, address);

        assert!(
            res.is_ok(),
            "Expected to read storage successfully. Got: {:?}",
            res
        );

        assert!(
            Felt::from_hex_unchecked("0xd501e7d0039aec6b") == res.unwrap(),
            "Expected storage value to match"
        );
    }

    #[test]
    fn test_sqlite_get_nonce_at() {
        let dir = std::env::current_dir().unwrap();
        let db_path = format!("sqlite://{}/mainnet-trimmed.sqlite", dir.to_str().unwrap());

        let reader =
            SQLiteStateReader::new_sqlite_state_reader(Url::parse(&db_path).unwrap(), 2002947);

        let contract_address = starknet_api::core::ContractAddress::try_from(
            starknet_core::types::Felt::from_hex_unchecked(
                "0x01a8b86c9bb05047b0136a96146c3a5bb5c806afa90687756be45341a86f8e37",
            ),
        )
        .unwrap();
        let res = reader.get_nonce_at(contract_address);

        assert!(
            res.is_ok(),
            "Expected to read nonce successfully. Got: {:?}",
            res
        );

        println!("Nonce: {:?}", res);

        assert!(
            res.unwrap().0 == starknet_core::types::Felt::from_hex_unchecked("0x2ad22"),
            "Expected nonce to match"
        );
    }

    #[test]
    fn test_sqlite_get_class_hash_at() {
        let dir = std::env::current_dir().unwrap();
        let db_path = format!("sqlite://{}/mainnet-trimmed.sqlite", dir.to_str().unwrap());

        let reader =
            SQLiteStateReader::new_sqlite_state_reader(Url::parse(&db_path).unwrap(), 1642947);

        let contract_address = starknet_api::core::ContractAddress::try_from(
            starknet_core::types::Felt::from_hex_unchecked(
                "0x049D36570D4e46f48e99674bd3fcc84644DdD6b96F7C741B1562B82f9e004dC7",
            ),
        )
        .unwrap();
        let res = reader.get_class_hash_at(contract_address);

        assert!(
            res.is_ok(),
            "Expected to read class hash successfully. Got: {:?}",
            res
        );
        assert!(
            res.unwrap()
                == starknet_api::core::ClassHash(starknet_core::types::Felt::from_hex_unchecked(
                    "0x07f3777c99f3700505ea966676aac4a0d692c2a9f5e667f4c606b51ca1dd3420"
                )),
            "Expected class hash to match"
        );
    }
}
