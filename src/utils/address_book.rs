use crate::{address, types::address::Address};

#[allow(dead_code)]
pub struct AddressBook {
    pub cartesi_app_factory: Address,
    pub app_address_relay: Address,
    pub erc1155_batch_portal: Address,
    pub erc1155_single_portal: Address,
    pub erc20_portal: Address,
    pub erc721_portal: Address,
    pub ether_portal: Address,
    pub input_box: Address,
}

impl AddressBook {
    pub fn default() -> Self {
        Self {
            cartesi_app_factory: address!("0x7122cd1221C20892234186facfE8615e6743Ab02"),
            app_address_relay: address!("0xF5DE34d6BbC0446E2a45719E718efEbaaE179daE"),
            erc1155_batch_portal: address!("0xedB53860A6B52bbb7561Ad596416ee9965B055Aa"),
            erc1155_single_portal: address!("0x7CFB0193Ca87eB6e48056885E026552c3A941FC4"),
            erc20_portal: address!("0x9C21AEb2093C32DDbC53eEF24B873BDCd1aDa1DB"),
            erc721_portal: address!("0x237F8DD094C0e47f4236f12b4Fa01d6Dae89fb87"),
            ether_portal: address!("0xFfdbe43d4c855BF7e0f105c400A50857f53AB044"),
            input_box: address!("0x59b22D57D4f067708AB0c00552767405926dc768"),
        }
    }
}
