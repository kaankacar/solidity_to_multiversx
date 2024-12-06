use crate::{transpiler::RustExpression, type_name::{ElementaryType, TypeName}};
use solang_parser::pt::{Expression, Type};
use anyhow::{Result, anyhow};


/*pub fn transform_visibility(visibility: Visibility) -> RustVisibility {
    match visibility {
        Visibility::Public(_) | Visibility::External(_) => RustVisibility::Public,
        Visibility::Private(_) | Visibility::Internal(_) => RustVisibility::Private,
    }
}*/

pub fn transform_expression(expr: &Expression) -> Result<RustExpression> {
    match expr {
        Expression::Variable(id) => Ok(RustExpression::Identifier(id.name.clone())),
        Expression::NumberLiteral(_, num, _, _) => Ok(RustExpression::NumberLiteral(num.clone())),
        Expression::StringLiteral(vals) => Ok(RustExpression::StringLiteral(
            vals.iter().map(|s| s.string.clone()).collect::<Vec<_>>().join("")
        )),
        Expression::BoolLiteral(_, value) => Ok(RustExpression::BooleanLiteral(*value)),
        Expression::Add(_, left, right) => Ok(RustExpression::BinaryOperation {
            left: Box::new(transform_expression(left)?),
            operator: "+".to_string(),
            right: Box::new(transform_expression(right)?),
        }),
        _ => Err(anyhow!("Unsupported expression type {:?}", expr)),
    }
}

pub fn convert_expression_to_type(expression: &Expression) -> Result<TypeName> {
    match expression {
        Expression::Type(_, ty) => match ty {
            Type::Address => Ok(TypeName::Elementary(ElementaryType::Address)),
            Type::Bool => Ok(TypeName::Elementary(ElementaryType::Bool)),
            Type::String => Ok(TypeName::Elementary(ElementaryType::String)),
            Type::Uint(size) => Ok(TypeName::Elementary(ElementaryType::Uint(*size))),
            Type::Int(size) => Ok(TypeName::Elementary(ElementaryType::Int(*size))),
            Type::Bytes(size) => Ok(TypeName::Elementary(ElementaryType::Bytes(Some(*size)))),
            _ => Err(anyhow!("Unsupported type")),
        },
        _ => Err(anyhow!("Unsupported expression type for type conversion")),
    }
}

