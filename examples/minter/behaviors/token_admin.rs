use std::{collections::HashMap, num::NonZero};

use starkbiter_bindings::{
    erc_20_mintable_oz0::Erc20MintableOZ0, ARGENT_v040_SIERRA, ERC20_CONTRACT_SIERRA,
};
use starkbiter_core::middleware::{connection::Connection, traits::Middleware};
use starknet_accounts::{Account, SingleOwnerAccount};
use starknet_core::{
    types::{Call, Felt},
    utils::get_selector_from_name,
};
use starknet_devnet_core::constants::UDC_CONTRACT_ADDRESS;
use starknet_devnet_types::rpc::gas_modification::GasModificationRequest;
use starknet_signers::{LocalWallet, SigningKey};

use super::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct TokenAdmin {
    /// The identifier of the token admin.
    pub token_data: HashMap<String, TokenData>,

    #[serde(skip)]
    pub account: Option<SingleOwnerAccount<Connection, LocalWallet>>,

    #[serde(skip)]
    pub tokens: HashMap<String, Erc20MintableOZ0<SingleOwnerAccount<Connection, LocalWallet>>>,
    #[serde(skip)]
    pub client: Option<Arc<StarkbiterMiddleware>>,
    #[serde(skip)]
    pub messager: Option<Messager>,
    #[serde(default)]
    pub count: u64,
    #[serde(default = "default_max_count")]
    pub max_count: Option<u64>,
}

pub fn default_max_count() -> Option<u64> {
    Some(3)
}

/// Used as an action to ask what tokens are available.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TokenAdminQuery {
    /// Get the address of the token.
    AddressOf(String),

    /// Mint tokens.
    MintRequest(MintRequest),
}

/// Used as an action to mint tokens.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MintRequest {
    /// The token to mint.
    pub token: String,

    /// The address to mint to.
    pub mint_to: Felt,

    /// The amount to mint.
    pub mint_amount: u64,
}

const ALL_GAS_1: GasModificationRequest = GasModificationRequest {
    gas_price_wei: NonZero::new(1_u128),
    gas_price_fri: NonZero::new(1_u128),

    data_gas_price_wei: NonZero::new(1_u128),
    data_gas_price_fri: NonZero::new(1_u128),

    l2_gas_price_wei: NonZero::new(1_u128),
    l2_gas_price_fri: NonZero::new(1_u128),

    generate_block: Some(true),
};

#[async_trait::async_trait]
impl Behavior<Message> for TokenAdmin {
    #[tracing::instrument(skip(self), fields(id = messager.id.as_deref()))]
    async fn startup(
        &mut self,
        client: Arc<StarkbiterMiddleware>,
        messager: Messager,
    ) -> Result<Option<EventStream<Message>>> {
        self.messager = Some(messager.clone());
        self.client = Some(client.clone());
        self.tokens = HashMap::new();

        client.set_next_block_gas(ALL_GAS_1).await?;

        let argent_class_hash = client.declare_contract(ARGENT_v040_SIERRA).await?;

        let account = &client
            .create_single_owner_account(
                Some(SigningKey::from_random()),
                argent_class_hash,
                10000000,
            )
            .await
            .unwrap();

        self.account = Some(account.clone());

        client.create_block().await?;

        let address = account.address().clone();

        let erc20_class_hash = client.declare_contract(ERC20_CONTRACT_SIERRA).await?;

        for token_data in self.token_data.values_mut() {
            let deploy_call = vec![Call {
                to: UDC_CONTRACT_ADDRESS,
                selector: get_selector_from_name("deployContract").unwrap(),
                calldata: vec![
                    erc20_class_hash,                  // class hash
                    Felt::from_hex_unchecked("0x123"), // salt
                    Felt::ZERO,                        // unique
                    Felt::ONE,                         // constructor length
                    address,                           // constructor arguments
                ],
            }];

            let result = account.execute_v3(deploy_call).send().await.unwrap();

            let token_contract_address = client
                .get_deployed_contract_address(result.transaction_hash)
                .await
                .unwrap();

            let token = Erc20MintableOZ0::new(token_contract_address, account.clone());

            token_data.address = Some(ContractAddress::from(token_contract_address.clone()));

            self.tokens.insert(token_data.name.clone(), token);
        }

        client.create_block().await?;

        Ok(Some(messager.stream()?))
    }

    #[tracing::instrument(skip(self), fields(id = self.messager.as_ref().unwrap().id.as_deref()))]
    async fn process(&mut self, event: Message) -> Result<ControlFlow> {
        if self.tokens.len() == 0 {
            error!(
                "There were no tokens to deploy! You must add tokens to
 the token admin before running the simulation."
            );
        }

        let query: TokenAdminQuery = serde_json::from_str(&event.data).unwrap();

        trace!("Got query: {:?}", query);
        let messager = self.messager.as_ref().unwrap();
        match query {
            TokenAdminQuery::AddressOf(token_name) => {
                trace!(
                    "Getting address of token with name: {:?}",
                    token_name.clone()
                );
                let token_data = self.token_data.get(&token_name).unwrap();

                trace!(
                    "Got address of token with name: {:?}. Result: {:?}",
                    token_name.clone(),
                    token_data.address.clone()
                );

                messager
                    .send(To::Agent(event.from.clone()), token_data)
                    .await?;
            }
            TokenAdminQuery::MintRequest(mint_request) => {
                trace!("Minting tokens: {:?}", mint_request);
                trace!(
                    "Self account: {:?}",
                    self.account.clone().unwrap().address()
                );
                let token = self.tokens.get(&mint_request.token).unwrap();

                let amount = cainome::cairo_serde::U256 {
                    low: mint_request.mint_amount.into(),
                    high: 0,
                };

                let result = token
                    .mint(
                        &cainome::cairo_serde::ContractAddress::from(mint_request.mint_to),
                        &amount,
                    )
                    .send()
                    .await
                    .unwrap();
                self.count += 1;

                if self.count == self.max_count.unwrap_or(u64::MAX) {
                    warn!("Reached max count. Halting behavior.");
                    return Ok(ControlFlow::Halt);
                }
            }
        }
        self.client.as_ref().unwrap().create_block().await?;
        Ok(ControlFlow::Continue)
    }
}
