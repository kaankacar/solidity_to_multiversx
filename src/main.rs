mod parser;
mod attribute_extraction;
mod transpiler;
mod helper_functions;
mod final_conversion;
mod type_mapper;
mod type_name;

use std::{env, fs::{self, File}, io::{self, Write}, path::Path};


use crate::final_conversion::convert_solidity_to_rust;
use anyhow::Result;

fn main() -> Result<()> {
    //let solidity_code = "contract TestContract { uint256 public value; function setValue(uint256 _value) public { value = _value; } }";


    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <input.sol>", args[0]);
        std::process::exit(1);
    }

    let input_file = &args[1];

    let solidity_code = fs::read_to_string(input_file)?;

    let rust_ast = convert_solidity_to_rust(&solidity_code)?;
    let output_file = format!("{}.rs", input_file.trim_end_matches(".sol"));
    let mut file = fs::File::create(&output_file)?;
    writeln!(file, "{}", rust_ast)?;


    
    Ok(())
}



