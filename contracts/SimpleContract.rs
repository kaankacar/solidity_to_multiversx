#![no_std]

use multiversx_sc::imports::*;

#[multiversx_sc::contract]
pub trait SimpleContract {
    #[storage_mapper("storedData")]
    pub fn stored_data(&self) -> SingleValueMapper<u256>;
}

impl SimpleContract {
    #[init]
    fn init(&self, initialValue: u256) {
        self.storedData().set(initialValue);
    }
    #[endpoint(setData)]
    fn set_data(&self, x: u256) {
        self.storedData().set(x);
    }
    #[view(get)]
    pub fn get(&self) -> u256 {
        self.stored_data().get()
    }
}

