pub mod argent_account;
pub mod contracts_counter;
pub mod contracts_router_lite;
pub mod contracts_swapper;
pub mod contracts_user_values;
pub mod ekubo_core;
pub mod erc_20_mintable_oz0;

// Saved contract compiled Sierra sources. Used for declaration of the
// contracts. Should be moved to cainome.
//

pub static COUNTER_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_Counter.contract_class.json");

pub static ERC20_CONTRACT_SIERRA: &str =
    include_str!("../contracts/ERC20_Mintable_OZ_0.8.1.class.json");

pub static ARGENT_V040_SIERRA: &str = include_str!("../contracts/ArgentAccount.0.4.0.class.json");

pub static SWAPPER_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_Swapper.contract_class.json");

pub static EKUBO_CORE_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_Swapper.contract_class.json");

pub static EKUBO_ROUTER_LITE_CONTRACT_SIERRA: &str =
    include_str!("../contracts/contracts_RouterLite.contract_class.json");
