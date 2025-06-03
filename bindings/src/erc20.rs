// Recursive expansion of abigen! macro for contracts_Counter.contract_class.json
// Some modifications were made:
// - added use alloc::string::String;
// - added use alloc::vec::Vec;
// - alloc::__export::must_use and just used formatted strings
// - removed #[boxc]
// =====================================

use alloc::string::String;
use alloc::vec::Vec;

#[derive()]
pub struct ERC20<A:starknet::accounts::ConnectedAccount+Sync>{
    pub address:starknet::core::types::Felt,pub account:A,pub block_id:starknet::core::types::BlockId,
}
impl <A:starknet::accounts::ConnectedAccount+Sync>ERC20<A>{
    pub fn new(address:starknet::core::types::Felt,account:A) -> Self {
        Self {
            address,account,block_id:starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending)
        }
    }
    pub fn set_contract_address(&mut self,address:starknet::core::types::Felt){
        self.address = address;
    }
    pub fn provider(&self) ->  &A::Provider {
        self.account.provider()
    }
    pub fn set_block(&mut self,block_id:starknet::core::types::BlockId){
        self.block_id = block_id;
    }
    pub fn with_block(self,block_id:starknet::core::types::BlockId) -> Self {
        Self {
            block_id, ..self
        }
    }

    }
#[derive()]
pub struct ERC20Reader<P:starknet::providers::Provider+Sync>{
    pub address:starknet::core::types::Felt,pub provider:P,pub block_id:starknet::core::types::BlockId,
}
impl <P:starknet::providers::Provider+Sync>ERC20Reader<P>{
    pub fn new(address:starknet::core::types::Felt,provider:P,) -> Self {
        Self {
            address,provider,block_id:starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending)
        }
    }
    pub fn set_contract_address(&mut self,address:starknet::core::types::Felt){
        self.address = address;
    }
    pub fn provider(&self) ->  &P {
        &self.provider
    }
    pub fn set_block(&mut self,block_id:starknet::core::types::BlockId){
        self.block_id = block_id;
    }
    pub fn with_block(self,block_id:starknet::core::types::BlockId) -> Self {
        Self {
            block_id, ..self
        }
    }

    }
#[derive()]
pub struct Approval {
    pub owner:cainome::cairo_serde::ContractAddress,pub spender:cainome::cairo_serde::ContractAddress,pub value:cainome::cairo_serde::U256
}
impl cainome::cairo_serde::CairoSerde for Approval {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.owner);
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.spender);
        __size+=cainome::cairo_serde::U256::cairo_serialized_size(&__rust.value);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        let mut __out:Vec<starknet::core::types::Felt>  = alloc::vec::Vec::new();
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.owner));
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.spender));
        __out.extend(cainome::cairo_serde::U256::cairo_serialize(&__rust.value));
        __out
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let mut __offset = __offset;
        let owner = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&owner);
        let spender = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&spender);
        let value = cainome::cairo_serde::U256::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
        Ok(Approval {
            owner,spender,value
        })
    }

    }
impl Approval {
    pub fn event_selector() -> starknet::core::types::Felt {
        starknet::core::utils::get_selector_from_name("Approval").unwrap()
    }
    pub fn event_name() ->  &'static str {
        "Approval"
    }

    }
#[derive()]
pub struct OwnershipTransferred {
    pub previous_owner:cainome::cairo_serde::ContractAddress,pub new_owner:cainome::cairo_serde::ContractAddress
}
impl cainome::cairo_serde::CairoSerde for OwnershipTransferred {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.previous_owner);
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.new_owner);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        let mut __out:Vec<starknet::core::types::Felt>  = alloc::vec::Vec::new();
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.previous_owner));
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.new_owner));
        __out
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let mut __offset = __offset;
        let previous_owner = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&previous_owner);
        let new_owner = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&new_owner);
        Ok(OwnershipTransferred {
            previous_owner,new_owner
        })
    }

    }
impl OwnershipTransferred {
    pub fn event_selector() -> starknet::core::types::Felt {
        starknet::core::utils::get_selector_from_name("OwnershipTransferred").unwrap()
    }
    pub fn event_name() ->  &'static str {
        "OwnershipTransferred"
    }

    }
#[derive()]
pub struct Transfer {
    pub from:cainome::cairo_serde::ContractAddress,pub to:cainome::cairo_serde::ContractAddress,pub value:cainome::cairo_serde::U256
}
impl cainome::cairo_serde::CairoSerde for Transfer {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        let mut __size = 0;
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.from);
        __size+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&__rust.to);
        __size+=cainome::cairo_serde::U256::cairo_serialized_size(&__rust.value);
        __size
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        let mut __out:Vec<starknet::core::types::Felt>  = alloc::vec::Vec::new();
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.from));
        __out.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(&__rust.to));
        __out.extend(cainome::cairo_serde::U256::cairo_serialize(&__rust.value));
        __out
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let mut __offset = __offset;
        let from = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&from);
        let to = cainome::cairo_serde::ContractAddress::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&to);
        let value = cainome::cairo_serde::U256::cairo_deserialize(__felts,__offset)? ;
        __offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
        Ok(Transfer {
            from,to,value
        })
    }

    }
impl Transfer {
    pub fn event_selector() -> starknet::core::types::Felt {
        starknet::core::utils::get_selector_from_name("Transfer").unwrap()
    }
    pub fn event_name() ->  &'static str {
        "Transfer"
    }

    }
#[derive()]
pub enum Event {
    OwnershipTransferred(OwnershipTransferred)
}
impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            Event::OwnershipTransferred(val) => OwnershipTransferred::cairo_serialized_size(val)+1,
            _ => 0
        
            }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        match __rust {
            Event::OwnershipTransferred(val) => {
                let mut temp = alloc::vec::Vec::new();
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(OwnershipTransferred::cairo_serialize(val));
                temp
            },
            _ => alloc::vec::Vec::new()
        
            }
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            0usize => Ok(Event::OwnershipTransferred(OwnershipTransferred::cairo_deserialize(__felts,__offset+1)?)),
            _ => return Err(cainome::cairo_serde::Error::Deserialize(format!("Index not handle for enum {}","Event")))
        
            }
    }

    }
impl TryFrom< &starknet::core::types::EmittedEvent>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::EmittedEvent) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("OwnershipTransferred").unwrap_or_else(|_|
            core::panicking::panic_fmt("Invalid selector for {}","OwnershipTransferred"));
        {
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let previous_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","previous_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&previous_owner);
            let new_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","new_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&new_owner);
            return Ok(Event::OwnershipTransferred(OwnershipTransferred {
                previous_owner,new_owner
            }))
        };
        Err(alloc::__export::must_use({
            let res = alloc::fmt::format(alloc::__export::format_args!("Could not match any event from keys {:?}",event.keys));
            res
        }))
    }

    }
impl TryFrom< &starknet::core::types::Event>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::Event) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("OwnershipTransferred").unwrap_or_else(|_|{
            core::panicking::panic_fmt("Invalid selector for {}","OwnershipTransferred");
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let previous_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","previous_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&previous_owner);
            let new_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","new_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&new_owner);
            return Ok(Event::OwnershipTransferred(OwnershipTransferred {
                previous_owner,new_owner
            }))
        };
        Err(format_args!("Could not match any event from keys {:?}", event.keys));
    }

    }
#[derive()]
pub enum Event {
    Transfer(Transfer),Approval(Approval)
}
impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            Event::Transfer(val) => Transfer::cairo_serialized_size(val)+1,
            Event::Approval(val) => Approval::cairo_serialized_size(val)+1,
            _ => 0
        
            }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        match __rust {
            Event::Transfer(val) => {
                let mut temp = alloc::vec::Vec::new();
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(Transfer::cairo_serialize(val));
                temp
            },
            Event::Approval(val) => {
                let mut temp = alloc::vec::Vec::new();
                temp.extend(usize::cairo_serialize(&1usize));
                temp.extend(Approval::cairo_serialize(val));
                temp
            },
            _ => alloc::vec::Vec::new()
        
            }
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            0usize => Ok(Event::Transfer(Transfer::cairo_deserialize(__felts,__offset+1)?)),
            1usize => Ok(Event::Approval(Approval::cairo_deserialize(__felts,__offset+1)?)),
            _ => return Err(cainome::cairo_serde::Error::Deserialize(format_args!("Index not handle for enum {}","Event")))
        
            }
    }

    }
impl TryFrom< &starknet::core::types::EmittedEvent>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::EmittedEvent) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Transfer").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Transfer"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let from = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","from","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&from);
            let to = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","to","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&to);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Transfer",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::Transfer(Transfer {
                from,to,value
            }))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Approval").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Approval"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","owner","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&owner);
            let spender = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","spender","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&spender);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::Approval(Approval {
                owner,spender,value
            }))
        };
        Err(alloc::__export::must_use({
            let res = alloc::fmt::format(alloc::__export::format_args!("Could not match any event from keys {:?}",event.keys));
            res
        }))
    }

    }
impl TryFrom< &starknet::core::types::Event>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::Event) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Transfer").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Transfer"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let from = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","from","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&from);
            let to = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","to","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&to);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Transfer",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::Transfer(Transfer {
                from,to,value
            }))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Approval").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Approval"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","owner","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&owner);
            let spender = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","spender","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&spender);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::Approval(Approval {
                owner,spender,value
            }))
        };
        Err(alloc::__export::must_use({
            let res = alloc::fmt::format(alloc::__export::format_args!("Could not match any event from keys {:?}",event.keys));
            res
        }))
    }

    }
#[derive()]
pub enum Event {
    ERC20Event(Event),OwnableEvent(Event)
}
impl cainome::cairo_serde::CairoSerde for Event {
    type RustType = Self;
    const SERIALIZED_SIZE:std::option::Option<usize>  = std::option::Option::None;
    #[inline]
    fn cairo_serialized_size(__rust: &Self::RustType) -> usize {
        match __rust {
            Event::ERC20Event(val) => Event::cairo_serialized_size(val)+1,
            Event::OwnableEvent(val) => Event::cairo_serialized_size(val)+1,
            _ => 0
        
            }
    }
    fn cairo_serialize(__rust: &Self::RustType) -> Vec<starknet::core::types::Felt>{
        match __rust {
            Event::ERC20Event(val) => {
                let mut temp = alloc::vec::Vec::new();
                temp.extend(usize::cairo_serialize(&0usize));
                temp.extend(Event::cairo_serialize(val));
                temp
            },
            Event::OwnableEvent(val) => {
                let mut temp = alloc::vec::Vec::new();
                temp.extend(usize::cairo_serialize(&1usize));
                temp.extend(Event::cairo_serialize(val));
                temp
            },
            _ => alloc::vec::Vec::new()
        
            }
    }
    fn cairo_deserialize(__felts: &[starknet::core::types::Felt],__offset:usize) -> cainome::cairo_serde::Result<Self::RustType>{
        let __f = __felts[__offset];
        let __index = u128::from_be_bytes(__f.to_bytes_be()[16..].try_into().unwrap());
        match __index as usize {
            0usize => Ok(Event::ERC20Event(Event::cairo_deserialize(__felts,__offset+1)?)),
            1usize => Ok(Event::OwnableEvent(Event::cairo_deserialize(__felts,__offset+1)?)),
            _ => return Err(cainome::cairo_serde::Error::Deserialize(format_args!("Index not handle for enum {}","Event")))

        }
    }

    }
impl TryFrom< &starknet::core::types::EmittedEvent>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::EmittedEvent) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Transfer").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Transfer"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let from = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","from","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&from);
            let to = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","to","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&to);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Transfer",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::ERC20Event(Event::Transfer(Transfer {
                from,to,value
            })))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Approval").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Approval"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","owner","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&owner);
            let spender = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","spender","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&spender);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::ERC20Event(Event::Approval(Approval {
                owner,spender,value
            })))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("OwnershipTransferred").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","OwnershipTransferred"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let previous_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","previous_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&previous_owner);
            let new_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","new_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&new_owner);
            return Ok(Event::OwnableEvent(Event::OwnershipTransferred(OwnershipTransferred {
                previous_owner,new_owner
            })))
        };
        Err(alloc::__export::must_use({
            let res = alloc::fmt::format(alloc::__export::format_args!("Could not match any event from keys {:?}",event.keys));
            res
        }))
    }

    }
impl TryFrom< &starknet::core::types::Event>for Event {
    type Error = String;
    fn try_from(event: &starknet::core::types::Event) -> Result<Self,Self::Error>{
        use cainome::cairo_serde::CairoSerde;
        if event.keys.is_empty(){
            return Err("Event has no key".to_string());
        }let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Transfer").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Transfer"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let from = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","from","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&from);
            let to = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.keys,key_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","to","Transfer",e));
                    res
                })),
            
                };
            key_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&to);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Transfer",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::ERC20Event(Event::Transfer(Transfer {
                from,to,value
            })))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("Approval").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","Approval"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","owner","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&owner);
            let spender = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","spender","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&spender);
            let value = match cainome::cairo_serde::U256::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","value","Approval",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::U256::cairo_serialized_size(&value);
            return Ok(Event::ERC20Event(Event::Approval(Approval {
                owner,spender,value
            })))
        };
        let selector = event.keys[0];
        if selector==starknet::core::utils::get_selector_from_name("OwnershipTransferred").unwrap_or_else(|_|{
            core::panicking::panic_fmt(core::const_format_args!("Invalid selector for {}","OwnershipTransferred"));
        }){
            let mut key_offset = 0+1;
            let mut data_offset = 0;
            let previous_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","previous_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&previous_owner);
            let new_owner = match cainome::cairo_serde::ContractAddress::cairo_deserialize(&event.data,data_offset){
                Ok(v) => v,
                Err(e) => return Err(alloc::__export::must_use({
                    let res = alloc::fmt::format(alloc::__export::format_args!("Could not deserialize field {} for {}: {:?}","new_owner","OwnershipTransferred",e));
                    res
                })),
            
                };
            data_offset+=cainome::cairo_serde::ContractAddress::cairo_serialized_size(&new_owner);
            return Ok(Event::OwnableEvent(Event::OwnershipTransferred(OwnershipTransferred {
                previous_owner,new_owner
            })))
        };
        Err(alloc::__export::must_use({
            let res = alloc::fmt::format(alloc::__export::format_args!("Could not match any event from keys {:?}",event.keys));
            res
        }))
    }

    }
impl <A:starknet::accounts::ConnectedAccount+Sync>ERC20<A>{
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn allowance(&self,owner: &cainome::cairo_serde::ContractAddress,spender: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(owner));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(spender));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([209766809471042785,12509652079980974336,17678706014116587206,10308380584186584568]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn balanceOf(&self,account: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(account));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([186492163330788704,9799122768618501063,3817639149632004388,8914400797191611589]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn balance_of(&self,account: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(account));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([343731218995559270,5264068538989673709,11305066377149084298,3187814280469066768]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn decimals(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,u8>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([451190754876481978,755531479336054762,12219577301920418346,15360137715940544477]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn name(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,starknet::core::types::Felt>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([204437639094763333,16059280649635539212,2380157814635835479,4539611826636167848]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn owner(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::ContractAddress>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([79001270948071610,6547467622138833813,7374318920648142905,11805883350411996556]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn symbol(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,starknet::core::types::Felt>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([179664498801601103,11796451914517155703,17130963829830369960,9178143007560336762]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn totalSupply(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([69716027446197474,7202072203292561311,16591666299386464833,18112103592448476716]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn total_supply(&self,) -> cainome::cairo_serde::call::FCall<A::Provider,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([208591859614417130,2722038587670665178,14634243038931256332,12095397922800688737]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn approve_getcall(&self,spender: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(spender));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([140600710993877394,13695063774359579960,7107368782042727669,12534173288940467319]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn approve(&self,spender: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(spender));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([140600710993877394,13695063774359579960,7107368782042727669,12534173288940467319]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn mint_getcall(&self,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([325361499047323408,1807990883272191924,6799885222290033850,14659918614400108700]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn mint(&self,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([325361499047323408,1807990883272191924,6799885222290033850,14659918614400108700]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn renounceOwnership_getcall(&self,) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([92818588173393328,2742434030733745590,4629040184120009675,8528297113810639112]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn renounceOwnership(&self,) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([92818588173393328,2742434030733745590,4629040184120009675,8528297113810639112]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn renounce_ownership_getcall(&self,) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([421193528422277900,870012963428109543,14662685225412515693,11861903034315795299]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn renounce_ownership(&self,) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([421193528422277900,870012963428109543,14662685225412515693,11861903034315795299]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_getcall(&self,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([437381113334062809,2507318034922653180,16370534037708042650,5927927059297104468]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer(&self,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([437381113334062809,2507318034922653180,16370534037708042650,5927927059297104468]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transferFrom_getcall(&self,sender: &cainome::cairo_serde::ContractAddress,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(sender));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([427849668778881624,11765991293598804043,3869861725982937294,12809927063331595464]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transferFrom(&self,sender: &cainome::cairo_serde::ContractAddress,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(sender));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([427849668778881624,11765991293598804043,3869861725982937294,12809927063331595464]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transferOwnership_getcall(&self,newOwner: &cainome::cairo_serde::ContractAddress) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(newOwner));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([111379915642854657,9673478759870692583,6827385907246002551,5640937904907466399]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transferOwnership(&self,newOwner: &cainome::cairo_serde::ContractAddress) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(newOwner));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([111379915642854657,9673478759870692583,6827385907246002551,5640937904907466399]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_from_getcall(&self,sender: &cainome::cairo_serde::ContractAddress,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(sender));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([464342347793828655,13262269167218251377,9348103406587822923,425915925847708578]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_from(&self,sender: &cainome::cairo_serde::ContractAddress,recipient: &cainome::cairo_serde::ContractAddress,amount: &cainome::cairo_serde::U256) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(sender));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(recipient));
        __calldata.extend(cainome::cairo_serde::U256::cairo_serialize(amount));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([464342347793828655,13262269167218251377,9348103406587822923,425915925847708578]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_ownership_getcall(&self,new_owner: &cainome::cairo_serde::ContractAddress) -> starknet::core::types::Call {
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(new_owner));
        starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([467546346665686576,2005871117359994763,18293666948765761729,17543713541245729446]),calldata:__calldata,
        }
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn transfer_ownership(&self,new_owner: &cainome::cairo_serde::ContractAddress) -> starknet::accounts::ExecutionV3<A>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(new_owner));
        let __call = starknet::core::types::Call {
            to:self.address,selector:::starknet::core::types::Felt::from_raw([467546346665686576,2005871117359994763,18293666948765761729,17543713541245729446]),calldata:__calldata,
        };
        self.account.execute_v3(<[_]>::into_vec(#[rustc_box]
        alloc::boxed::Box::new([__call])))
    }

    }
impl <P:starknet::providers::Provider+Sync>ERC20Reader<P>{
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn allowance(&self,owner: &cainome::cairo_serde::ContractAddress,spender: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(owner));
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(spender));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([209766809471042785,12509652079980974336,17678706014116587206,10308380584186584568]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn balanceOf(&self,account: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(account));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([186492163330788704,9799122768618501063,3817639149632004388,8914400797191611589]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn balance_of(&self,account: &cainome::cairo_serde::ContractAddress) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        __calldata.extend(cainome::cairo_serde::ContractAddress::cairo_serialize(account));
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([343731218995559270,5264068538989673709,11305066377149084298,3187814280469066768]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn decimals(&self,) -> cainome::cairo_serde::call::FCall<P,u8>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([451190754876481978,755531479336054762,12219577301920418346,15360137715940544477]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn name(&self,) -> cainome::cairo_serde::call::FCall<P,starknet::core::types::Felt>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([204437639094763333,16059280649635539212,2380157814635835479,4539611826636167848]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn owner(&self,) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::ContractAddress>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([79001270948071610,6547467622138833813,7374318920648142905,11805883350411996556]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn symbol(&self,) -> cainome::cairo_serde::call::FCall<P,starknet::core::types::Felt>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([179664498801601103,11796451914517155703,17130963829830369960,9178143007560336762]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn totalSupply(&self,) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([69716027446197474,7202072203292561311,16591666299386464833,18112103592448476716]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }
    #[allow(clippy::ptr_arg)]
    #[allow(clippy::too_many_arguments)]
    pub fn total_supply(&self,) -> cainome::cairo_serde::call::FCall<P,cainome::cairo_serde::U256>{
        use cainome::cairo_serde::CairoSerde;
        let mut __calldata = alloc::vec::Vec::new();
        let __call = starknet::core::types::FunctionCall {
            contract_address:self.address,entry_point_selector:::starknet::core::types::Felt::from_raw([208591859614417130,2722038587670665178,14634243038931256332,12095397922800688737]),calldata:__calldata,
        };
        cainome::cairo_serde::call::FCall::new(__call,self.provider(),)
    }

    }