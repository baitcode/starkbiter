//! This module is a wrapper around the bridged token data from starkgate.io
//! bridged tokens data JSON files.
//!
//! It exports `get_token_data` that returns the bridged token data for a given
//! TokenId TokenId is exported as well. This is intended to be used with forked
//! Starknet Devnet Mainnet and Sepolia networks.

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use starknet_core::types::Felt;
use starknet_devnet_types::chain_id::ChainId;

static MAINNET_JSON: &str = include_str!("./assets/mainnet.json");
static SEPOLIA_JSON: &str = include_str!("./assets/sepolia.json");

/// Represents the supported token identifiers.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum TokenId {
    STRK,
    ETH,
    USDC,
    USDT,
    DAI,
    EKUBO,
}

impl From<&TokenId> for String {
    fn from(value: &TokenId) -> Self {
        match value {
            TokenId::STRK => "strk".to_string(),
            TokenId::ETH => "eth".to_string(),
            TokenId::USDC => "usdc".to_string(),
            TokenId::USDT => "usdt".to_string(),
            TokenId::DAI => "dai".to_string(),
            TokenId::EKUBO => "ekubo".to_string(),
        }
    }
}

impl TryFrom<&str> for TokenId {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "strk" => Ok(TokenId::STRK),
            "eth" => Ok(TokenId::ETH),
            "usdc" => Ok(TokenId::USDC),
            "usdt" => Ok(TokenId::USDT),
            "dai" => Ok(TokenId::DAI),
            "ekubo" => Ok(TokenId::EKUBO),
            _ => Err(anyhow!("Unsupported token identifier: {}", value)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct SerialisedTokenData {
    id: Option<String>,
    name: String,
    symbol: Option<String>,
    decimals: Option<u8>,
    l1_token_address: Option<Felt>,
    l1_bridge_address: Option<Felt>,
    l2_bridge_address: Option<Felt>,
    l2_token_address: Option<Felt>,
}

/// Represents a bridged token with its associated metadata and addresses.
#[derive(Clone, Debug)]
pub struct BridgedToken {
    /// The unique identifier for the token.
    pub id: String,
    /// The name of the token.
    pub name: String,
    /// The symbol of the token.
    pub symbol: String,
    /// The number of decimal places the token can be divided into.
    pub decimals: u8,
    /// The address of the token on the L1 (Layer 1) network.
    pub l1_token_address: Felt,
    /// The address of the bridge contract on the L1 network.
    pub l1_bridge_address: Felt,
    /// The address of the bridge contract on the L2 (Layer 2) network.
    pub l2_bridge_address: Felt,
    /// The address of the token on the L2 network.
    pub l2_token_address: Felt,
}

type BridgedTokenData = HashMap<String, BridgedToken>;

struct BridgedTokenDataStorage {
    _cache: HashMap<ChainId, BridgedTokenData>,
}

fn load_data(source: &str) -> BridgedTokenData {
    let deserialised: Vec<SerialisedTokenData> =
        serde_json::from_str(source).expect("Failed to parse token data");

    deserialised
        .into_iter()
        .filter(|t| {
            t.id.is_some()
                && t.symbol.is_some()
                && t.decimals.is_some()
                && t.l1_token_address.is_some()
                && t.l1_bridge_address.is_some()
                && t.l2_bridge_address.is_some()
                && t.l2_token_address.is_some()
        })
        .map(|t| {
            (
                t.id.clone().unwrap(),
                BridgedToken {
                    id: t.id.unwrap(),
                    name: t.name,
                    symbol: t.symbol.unwrap(),
                    decimals: t.decimals.unwrap(),
                    l1_token_address: t.l1_token_address.unwrap(),
                    l1_bridge_address: t.l1_bridge_address.unwrap(),
                    l2_bridge_address: t.l2_bridge_address.unwrap(),
                    l2_token_address: t.l2_token_address.unwrap(),
                },
            )
        })
        .collect::<HashMap<String, BridgedToken>>()
}

impl BridgedTokenDataStorage {
    pub fn new() -> Self {
        Self {
            _cache: HashMap::new(),
        }
    }

    pub fn get(&mut self, chain_id: &ChainId, token_id: &TokenId) -> Option<&BridgedToken> {
        match chain_id {
            ChainId::Mainnet => {
                let data = self
                    ._cache
                    .entry(*chain_id)
                    .or_insert_with(|| load_data(MAINNET_JSON));

                let key = String::from(token_id);
                data.get(&key)
            }
            ChainId::Testnet => {
                let data = self
                    ._cache
                    .entry(*chain_id)
                    .or_insert_with(|| load_data(SEPOLIA_JSON));

                data.get(&String::from(token_id))
            }
            _ => unimplemented!("Only supports data for Mainnet and Testnet (sepolia)"),
        }
    }
}

static INSTANCE: OnceLock<Mutex<BridgedTokenDataStorage>> = OnceLock::new();
fn cache() -> &'static Mutex<BridgedTokenDataStorage> {
    INSTANCE.get_or_init(|| Mutex::new(BridgedTokenDataStorage::new()))
}

/// Initialises token data into cache singleton and retrieves the bridged token
/// data for the given chain ID and token ID.
///
/// # Panics
///
/// Panics if the token data is not found o
pub fn get_token_data(chain_id: &ChainId, token_id: &TokenId) -> Result<BridgedToken> {
    cache()
        .lock()
        .unwrap()
        .get(chain_id, token_id)
        .cloned()
        .ok_or(anyhow!(
            "Token data not found for chain ID: {:?} and token ID: {:?}",
            chain_id,
            token_id
        ))
}
