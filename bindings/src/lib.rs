pub mod argent_account;
pub mod contracts_counter;
pub mod contracts_user_values;
pub mod erc_20_mintable_oz0;

pub static COUNTER_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_Counter.contract_class.json");

static USER_VALUES_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_UserValues.contract_class.json");

pub static ERC20_CONTRACT_SIERRA: &str =
    include_str!("../contracts/ERC20_Mintable_OZ_0.8.1.class.json");

pub static ARGENT_v040_SIERRA: &str = include_str!("../contracts/ArgentAccount.0.4.0.class.json");
