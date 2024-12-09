mod parser;
mod transpiler;
mod helper_functions;
mod final_conversion;
mod type_mapper;
mod type_name;
mod rust_ast;
mod statement;

use std::{env, fs::{self}, io::Write};


use crate::final_conversion::convert_solidity_to_rust;
use anyhow::Result;

fn main() -> Result<()> {

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



