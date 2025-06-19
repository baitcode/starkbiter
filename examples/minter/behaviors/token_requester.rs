use std::num::NonZero;

use starkbiter_bindings::{
    erc_20_mintable_oz0::{ERC20ComponentEvent, Erc20MintableOZ0},
    ARGENT_v040_SIERRA,
};
use starkbiter_core::middleware::traits::Middleware;

use starknet_accounts::{Account, SingleOwnerAccount};

use starknet_core::types::Felt;
use starknet_devnet_types::rpc::gas_modification::GasModificationRequest;

use super::*;

use starknet_signers::SigningKey;
use token_admin::{MintRequest, TokenAdminQuery};

/// The token requester is responsible for requesting tokens from the token
/// admin. This agents is purely for testing purposes as far as I can tell.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct TokenRequester {
    /// The tokens that the token requester has requested.
    pub token_data: TokenData,
    /// The agent ID to request tokens to.
    pub request_to: String,

    pub address: Option<Felt>,
    /// Client to have an address to receive token mint to and check balance
    #[serde(skip)]
    pub client: Option<Arc<StarkbiterMiddleware>>,
    /// The messaging layer for the token requester.
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

type ERC20Contract = Erc20MintableOZ0<
    SingleOwnerAccount<
        starkbiter_core::middleware::connection::Connection,
        starknet_signers::LocalWallet,
    >,
>;

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
impl Behavior<ERC20ComponentEvent> for TokenRequester {
    #[tracing::instrument(skip(self), fields(id = messager.id.as_deref()))]
    async fn startup(
        &mut self,
        client: Arc<StarkbiterMiddleware>,
        mut messager: Messager,
    ) -> Result<Option<EventStream<ERC20ComponentEvent>>> {
        messager
            .send(
                To::Agent(self.request_to.clone()),
                &TokenAdminQuery::AddressOf(self.token_data.name.clone()),
            )
            .await?;

        client.set_next_block_gas(ALL_GAS_1).await?;

        let message = messager.get_next().await.unwrap();
        self.token_data = serde_json::from_str::<TokenData>(&message.data).unwrap();

        let argent_class_hash = client.declare_contract(ARGENT_v040_SIERRA).await?;

        let account = client
            .create_single_owner_account(
                Some(SigningKey::from_random()),
                argent_class_hash,
                10000000,
            )
            .await?;

        self.address = Some(account.address());

        let mint_data = TokenAdminQuery::MintRequest(MintRequest {
            token: self.token_data.name.clone(),
            mint_to: self.address.unwrap(),
            mint_amount: 1,
        });

        messager
            .send(To::Agent(self.request_to.clone()), mint_data)
            .await?;

        self.messager = Some(messager.clone());
        self.client = Some(client.clone());

        client.create_block().await?;

        let subscribe_stream = client.subscribe_to_flatten::<ERC20ComponentEvent>().await;

        Ok(Some(subscribe_stream))
    }

    #[tracing::instrument(skip(self), fields(id = self.messager.as_ref().unwrap().id.as_deref()))]
    async fn process(&mut self, _event: ERC20ComponentEvent) -> Result<ControlFlow> {
        let messager = self.messager.as_ref().unwrap();

        while self.count < self.max_count.unwrap() {
            debug!("sending message from requester");
            let mint_data = TokenAdminQuery::MintRequest(MintRequest {
                token: self.token_data.name.clone(),
                mint_to: self.address.unwrap(),
                mint_amount: 1,
            });

            messager
                .send(To::Agent(self.request_to.clone()), mint_data)
                .await?;
            self.count += 1;
        }
        warn!("Reached max count. Halting behavior.");
        Ok(ControlFlow::Halt)
    }
}
