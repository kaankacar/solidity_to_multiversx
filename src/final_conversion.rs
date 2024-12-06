use crate::parser::ParsedContract;
use crate::attribute_extraction::extract_multiversx_attributes;
use crate::transpiler::{transform_with_attributes, RustNode};
use anyhow::Result;

pub fn convert_solidity_to_rust(source: &str) -> Result<String> {
    let parsed_contract = crate::parser::parse_contract(source)?;
    let attributes = extract_multiversx_attributes(source)?;
    let rust_ast = transform_with_attributes(parsed_contract, attributes)?;
    
    Ok(rust_ast)
}