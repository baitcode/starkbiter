// Recursive expansion of abigen! macro
// =====================================

use alloc::string::String;
use alloc::vec::Vec;

#[derive()]
pub struct Counter<A: starknet::accounts::ConnectedAccount + Sync> {
    pub address: starknet::core::types::Felt,
    pub account: A,
    pub block_id: starknet::core::types::BlockId,
}
impl<A: starknet::accounts::ConnectedAccount + Sync> Counter<A> {
    pub fn new(address: starknet::core::types::Felt, account: A) -> Self {
        Self {
            address,
            account,
            block_id: starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending),
        }
    }
    pub fn set_contract_address(&mut self, address: starknet::core::types::Felt) {
        self.address = address;
    }
    pub fn provider(&self) -> &A::Provider {
        self.account.provider()
    }
    pub fn set_block(&mut self, block_id: starknet::core::types::BlockId) {
        self.block_id = block_id;
    }
    pub fn with_block(self, block_id: starknet::core::types::BlockId) -> Self {
        Self { block_id, ..self }
    }
}
#[derive()]
pub struct CounterReader<P: starknet::providers::Provider + Sync> {
    pub address: starknet::core::types::Felt,
    pub provider: P,
    pub block_id: starknet::core::types::BlockId,
}
impl<P: starknet::providers::Provider + Sync> CounterReader<P> {
    pub fn new(address: starknet::core::types::Felt, provider: P) -> Self {
        Self {
            address,
            provider,
            block_id: starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending),
        }
    }
    pub fn set_contract_address(&mut self, address: starknet::core::types::Felt) {
        self.address = address;
    }
    pub fn provider(&self) -> &P {
        &self.provider
    }
    pub fn set_block(&mut self, block_id: starknet::core::types::BlockId) {
        self.block_id = block_id;
    }
    pub fn with_block(self, block_id: starknet::core::types::BlockId) -> Self {
        Self { block_id, ..self }
    }
}
#[derive()]
pub enum Event {}

impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE: std::option::Option<usize> = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            _ => 0,
        }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt> {
        match __rust {
            _ => alloc::vec::Vec::new(),
        }
    }
    fn cairo_deserialize(
        __felts: &[starknet::core::types::Felt],
        __offset: usize,
    ) -> cainome::cairo_serde::Result<Self::RustType> {
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            _ => {
                return Err(cainome::cairo_serde::Error::Deserialize(
                    alloc::fmt::format(alloc::__export::format_args!(
                        "Index not handle for enum {}",
                        "Event"
                    )),
                ))
            }
        }
    }
}
impl TryFrom<&starknet::core::types::EmittedEvent> for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::EmittedEvent) -> Result<Self, Self::Error> {
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty() {
            return Err("Event has no key".to_string());
        }
        Err(alloc::fmt::format(alloc::__export::format_args!(
            "Could not match any event from keys {:?}",
            event.keys
        )))
    }
}
impl TryFrom<&starknet::core::types::Event> for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::Event) -> Result<Self, Self::Error> {
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty() {
            return Err("Event has no key".to_string());
        }
        Err(alloc::fmt::format(alloc::__export::format_args!(
            "Could not match any event from keys {:?}",
            event.keys
        )))
    }
}
impl<A: starknet::accounts::ConnectedAccount + Sync> Counter<A> {
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get(
        &self,
        address: &cainome::cairo_serde::ContractAddress,
    ) -> cainome::cairo_serde::call::FCall<A::Provider, u64> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(
            address,
        ));
        let __call = starknet::core::types::FunctionCall {
            contract_address: self.address,
            entry_point_selector: ::starknet::core::types::Felt::from_raw([
                323917239471961004,
                11340057472575452111,
                3701648891119744781,
                12476408022399417873,
            ]),
            calldata: __calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call, self.provider())
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set_getcall(&self, amount: &u64) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(u64::cairo_serialize(amount));
        starknet::core::types::Call {
            to: self.address,
            selector: ::starknet::core::types::Felt::from_raw([
                407497293008696574,
                7658200186928249810,
                17936468831007992439,
                2663871288560670526,
            ]),
            calldata: __calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn set(&self, amount: &u64) -> starknet::accounts::ExecutionV3<A> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(u64::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to: self.address,
            selector: ::starknet::core::types::Felt::from_raw([
                407497293008696574,
                7658200186928249810,
                17936468831007992439,
                2663871288560670526,
            ]),
            calldata: __calldata,
        };
        self.account
            .execute_v3(<[_]>::into_vec(alloc::boxed::Box::new([__call])))
    }
}
impl<P: starknet::providers::Provider + Sync> CounterReader<P> {
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn get(
        &self,
        address: &cainome::cairo_serde::ContractAddress,
    ) -> cainome::cairo_serde::call::FCall<P, u64> {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(
            address,
        ));
        let __call = starknet::core::types::FunctionCall {
            contract_address: self.address,
            entry_point_selector: ::starknet::core::types::Felt::from_raw([
                323917239471961004,
                11340057472575452111,
                3701648891119744781,
                12476408022399417873,
            ]),
            calldata: __calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call, self.provider())
    }
}
