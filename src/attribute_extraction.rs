use anyhow::{anyhow, Result};
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "solidity.pest"] // Path to your grammar file
pub struct SolidityParser;

#[derive(Debug)]
pub enum MultiversXAttribute {
    Payable,
    View,
    Endpoint(String),
    Storage(String),
    Module(String),
    Proxy,
    Event,
}

pub fn extract_multiversx_attributes(source: &str) -> Result<Vec<MultiversXAttribute>> {
    let pairs = SolidityParser::parse(Rule::file, source)
        .map_err(|e| anyhow!("Failed to parse MultiversX attributes: {}", e))?;

    let mut attributes = Vec::new();

    for pair in pairs {
        if let Rule::multiversx_attribute = pair.as_rule() {
            attributes.push(parse_multiversx_attribute(pair)?);
        }
    }
    Ok(attributes)
}

fn parse_multiversx_attribute(pair: pest::iterators::Pair<'_, Rule>) -> Result<MultiversXAttribute> {
    let content = pair.as_str();
    if content.contains("view") {
        Ok(MultiversXAttribute::View)
    } else if content.contains("payable") {
        Ok(MultiversXAttribute::Payable)
    } else if content.contains("proxy") {
        Ok(MultiversXAttribute::Proxy)
    } else if content.contains("event") {
        Ok(MultiversXAttribute::Event)
    } else if let Some(name) = extract_attribute_name(content, "endpoint") {
        Ok(MultiversXAttribute::Endpoint(name))
    } else if let Some(name) = extract_attribute_name(content, "storage_mapper") {
        Ok(MultiversXAttribute::Storage(name))
    } else if let Some(name) = extract_attribute_name(content, "module") {
        Ok(MultiversXAttribute::Module(name))
    } else {
        Err(anyhow!("Unknown MultiversX attribute"))
    }
}

fn extract_attribute_name(content: &str, attribute: &str) -> Option<String> {
    let pattern = format!(r#"{}\("([^"]+)"\)"#, attribute);
    let re = regex::Regex::new(&pattern).ok()?;
    re.captures(content).and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
