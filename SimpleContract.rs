#![no_std]

use multiversx_sc::imports::*;

#[multiversx_sc::contract]
pub trait SimpleContract {
    #[storage_mapper("storedData")]
    pub fn storedData(&self) -> SingleValueMapper<u64>;

}

impl SimpleContract {
    // Constructor (handled differently)
    pub fn unnamed(&self, initialValue: u64) {
        storedData.set(initialValue);
    }

    #[endpoint]
    pub fn set(&self, x: u64) {
        storedData.set(x);
        // Unsupported statement
    }

    #[endpoint]
    pub fn get(&self, ) {
        return storedData; // Convert expression
    }

}
