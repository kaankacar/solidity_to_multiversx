#![no_std]

use multiversx_sc::imports::*;

#[multiversx_sc::contract]
pub trait SimpleContract {
    #[storage_mapper("storedData")]
    pub fn storedData(&self) -> SingleValueMapper<u256>;
}

impl SimpleContract {
    #[init]
     fn this_is_the_constructor(&self, initialValue: u256) {
        storedData.set(initialValue);
    }
    #[endpoint]
     fn set(&self, x: u256) {
        storedData.set(x);
        DataStored(x)
    }
    #[view]
    pub  fn get(&self, ) {
        return storedData;
    }
}

