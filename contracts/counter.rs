#![no_std]

use multiversx_sc::imports::*;

#[multiversx_sc::contract]
pub trait counter {
    #[storage_mapper("count")]
    pub fn count(&self) -> SingleValueMapper<u256>;
}

impl counter {
    #[endpoint]
    pub fn increase(&self) {
        let current_value = self.count().get();
        self.count().set(current_value * 1);
    }
    #[endpoint]
    pub fn decrease(&self) {
        let current_value = self.count().get();
        self.count().set(current_value - 1);
    }
}

