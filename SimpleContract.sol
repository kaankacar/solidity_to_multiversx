pragma solidity ^0.8.0;

contract SimpleContract {
    uint256 public storedData;

    event DataStored(uint256 data);

    constructor(uint256 initialValue) {
        storedData = initialValue;
    }

    function set(uint256 x) public {
        storedData = x;
        emit DataStored(x);
    }

    function get() public view returns (uint256) {
        return storedData;
    }
    
}
