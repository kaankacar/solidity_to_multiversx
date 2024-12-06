use crate::type_name::{TypeName, ElementaryType};
use anyhow::{Result, anyhow};

pub fn map_type(type_name: &TypeName) -> Result<String> {
    match type_name {
        TypeName::Elementary(elem_ty) => match elem_ty {
            ElementaryType::Address => Ok("AddressMapper".to_string()),
            ElementaryType::Bool => Ok("bool".to_string()),
            ElementaryType::String => Ok("ManagedBuffer".to_string()),
            ElementaryType::Uint(size) => Ok(format!("u{}", size)),
            ElementaryType::Int(size) => Ok(format!("i{}", size)),
            ElementaryType::Bytes(size) => Ok(format!("[u8; {}]", size.unwrap_or(32))),
        },
        TypeName::Array(base_ty, size) => Ok(format!("Vec<{}>", map_type(base_ty)?)),
        TypeName::Mapping { key_type, value_type } => Ok(format!("HashMap<{}, {}>", map_type(key_type)?, map_type(value_type)?)),
        TypeName::UserDefined(name) => Ok(name.clone()),
        _ => Err(anyhow!("Unsupported type")),
    }
}