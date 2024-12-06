use crate::attribute_extraction::MultiversXAttribute;
use crate::helper_functions::{
    convert_expression_to_type, transform_expression, transform_visibility,
};
use crate::parser::ParsedContract;
use crate::type_mapper::map_type;
use anyhow::{anyhow, Result};
use solang_parser::pt;

#[derive(Debug, Clone)]
pub enum RustExpression {
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    BooleanLiteral(bool),
    BinaryOperation {
        left: Box<RustExpression>,
        operator: String,
        right: Box<RustExpression>,
    },
    FunctionCall {
        function: Box<RustExpression>,
        arguments: Vec<RustExpression>,
    },
    MemberAccess {
        expression: Box<RustExpression>,
        member: String,
    },
}

#[derive(Debug)]
pub enum RustNode {
    Contract {
        name: String,
        body: Vec<RustNode>,
    },
    Function {
        name: String,
        params: Vec<RustParameter>,
        returns: Option<Vec<RustParameter>>,
        body: Vec<RustNode>,
        visibility: RustVisibility,
        is_endpoint: bool,
        is_view: bool,
    },
    StorageDefinition {
        name: String,
        type_name: String,
        visibility: RustVisibility,
    },
    Expression(RustExpression),
    Return(Option<RustExpression>),
    Assignment {
        target: Box<RustExpression>,
        value: Box<RustExpression>,
    },
    IfStatement {
        condition: Box<RustExpression>,
        body: Vec<RustNode>,
        else_body: Option<Vec<RustNode>>,
    },
    WhileStatement {
        condition: Box<RustExpression>,
        body: Vec<RustNode>,
    },
}

#[derive(Debug)]
pub struct RustParameter {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RustVisibility {
    Public,
    Private,
}

pub fn transform_with_attributes(
    parsed: ParsedContract,
    attributes: Vec<MultiversXAttribute>,
) -> Result<String> {
    let mut output = String::new();

    // Add header
    output.push_str("#![no_std]\n\n");
    output.push_str("use multiversx_sc::imports::*;\n\n");

    for part in parsed.solidity_ast.0 {
        if let pt::SourceUnitPart::ContractDefinition(contract) = part {
            let contract_name = contract
                .name
                .as_ref()
                .map(|name| name.name.clone())
                .unwrap_or_default();

            // Generate trait for storage mappers
            output.push_str(&format!(
                "#[multiversx_sc::contract]\npub trait {} {{\n",
                contract_name
            ));
            let rust_body = transform_contract_with_attributes(&contract, &attributes)?;
            for node in &rust_body {
                if let RustNode::StorageDefinition {
                    name, type_name, ..
                } = node
                {
                    output.push_str(&format!(
                        "    #[storage_mapper(\"{}\")]\n    pub fn {}(&self) -> SingleValueMapper<{}>;\n",
                        name, name, type_name
                    ));
                }
            }
            output.push_str("}\n\n");

            // Generate implementation
            output.push_str(&format!("impl {} {{\n", contract_name));
            for node in rust_body {
                if let RustNode::Function {
                    name,
                    params,
                    body,
                    is_endpoint,
                    is_view,
                    visibility,
                    ..
                } = node
                {
                    // Add annotation (endpoint or view)
                    let annotation = if is_endpoint {
                        "    #[endpoint]\n"
                    } else if is_view {
                        "    #[view]\n"
                    } else if name.is_empty() {
                        "    #[init]\n"
                    } else {
                        ""
                    };
                    output.push_str(annotation);

                    // Add function signature
                    let function_name = if name.is_empty() {
                        "this_is_the_constructor"
                    } else {
                        &name
                    };
                    let function_visibility = if visibility == RustVisibility::Private {
                        ""
                    } else {
                        "pub "
                    };

                    output.push_str(&format!(
                        "    {} fn {}(&self",
                        function_visibility, function_name
                    ));
                    let params_str = params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.type_name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    output.push_str(&format!(", {}) {{\n", params_str));

                    // Add function body
                    for stmt in body {
                        output.push_str(&format!(
                            "        {}\n",
                            transform_rust_node_to_code(&stmt)?
                        ));
                    }

                    output.push_str("    }\n");
                }
            }
            output.push_str("}\n");
        }
    }

    Ok(output)
}

fn transform_rust_node_to_code(node: &RustNode) -> Result<String> {
    match node {
        RustNode::Expression(expr) => match expr {
            RustExpression::MemberAccess { expression, member } => {
                let expr_code =
                    transform_rust_node_to_code(&RustNode::Expression(*expression.clone()))?;
                Ok(format!("{}.{}", expr_code, member))
            }
            RustExpression::FunctionCall {
                function,
                arguments,
            } => {
                let func_code =
                    transform_rust_node_to_code(&RustNode::Expression(*function.clone()))?;
                let args_code = arguments
                    .iter()
                    .map(|arg| transform_rust_node_to_code(&RustNode::Expression(arg.clone())))
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");
                Ok(format!("{}({})", func_code, args_code))
            }
            RustExpression::Identifier(name) => Ok(name.clone()),
            _ => Ok("// Unsupported expression".to_string()),
        },
        RustNode::Return(Some(expr)) => {
            let expr_code = transform_rust_node_to_code(&RustNode::Expression((*expr).clone()))?;
            Ok(format!("return {};", expr_code))
        }
        RustNode::Return(None) => Ok("return;".to_string()),
        RustNode::Assignment { target, value } => {
            let target_code = transform_rust_node_to_code(&RustNode::Expression(*target.clone()))?;
            let value_code = transform_rust_node_to_code(&RustNode::Expression(*value.clone()))?;
            Ok(format!("{}.set({});", target_code, value_code))
        }
        RustNode::Function {
            name,
            params,
            body,
            ..
        } if name == "block" => {
            // Handle "block" as inline statements
            let body_code = body
                .iter()
                .map(transform_rust_node_to_code)
                .collect::<Result<Vec<_>>>()?
                .join("\n        ");
            Ok(body_code)
        }
        RustNode::Function {
            name,
            params,
            body,
            ..
        } => {
            let params_code = params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.type_name))
                .collect::<Vec<_>>()
                .join(", ");
            let body_code = body
                .iter()
                .map(transform_rust_node_to_code)
                .collect::<Result<Vec<_>>>()?
                .join("\n        ");
            Ok(format!(
                "fn {}({}) {{\n        {}\n    }}",
                name, params_code, body_code
            ))
        }
        _ => {
            Ok("// Unsupported statement 3".to_string())
        }
    }
}

fn transform_contract_with_attributes(
    contract: &pt::ContractDefinition,
    attributes: &[MultiversXAttribute],
) -> Result<Vec<RustNode>> {
    let mut rust_nodes = Vec::new();
    for part in &contract.parts {
        match part {
            pt::ContractPart::VariableDefinition(var) => {
                let name = var
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .unwrap_or_default();
                let type_name = map_type(&convert_expression_to_type(&var.ty)?)?;
                rust_nodes.push(RustNode::StorageDefinition {
                    name,
                    type_name,
                    visibility: RustVisibility::Public, // Assuming public for simplicity
                });
            }
            pt::ContractPart::FunctionDefinition(func) => {
                let is_endpoint = attributes
                    .iter()
                    .any(|attr| matches!(attr, MultiversXAttribute::Endpoint(_)));
                let is_view = attributes
                    .iter()
                    .any(|attr| matches!(attr, MultiversXAttribute::View));
                let is_payable = attributes
                    .iter()
                    .any(|attr| matches!(attr, MultiversXAttribute::Payable));

                rust_nodes.push(transform_function_with_attributes(func)?);
            }
            _ => continue,
        }
    }
    Ok(rust_nodes)
}

fn transform_function_with_attributes(func: &pt::FunctionDefinition) -> Result<RustNode> {
    let params = func
        .params
        .iter()
        .filter_map(|(_, param)| param.as_ref())
        .map(|p| {
            let type_name = convert_expression_to_type(&p.ty)?;
            Ok(RustParameter {
                name: p
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .unwrap_or_default(),
                type_name: map_type(&type_name)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let returns = if func.returns.is_empty() {
        None
    } else {
        Some(
            func.returns
                .iter()
                .filter_map(|(_, param)| param.as_ref())
                .map(|p| {
                    let type_name = convert_expression_to_type(&p.ty)?;
                    Ok(RustParameter {
                        name: p
                            .name
                            .as_ref()
                            .map(|id| id.name.clone())
                            .unwrap_or_default(),
                        type_name: map_type(&type_name)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        )
    };

    // Determine visibility
    let visibility = func
        .attributes
        .iter()
        .find_map(|attr| {
            if let pt::FunctionAttribute::Visibility(vis) = attr {
                match vis {
                    pt::Visibility::Public(_) => Some(RustVisibility::Public),
                    pt::Visibility::Internal(_) | pt::Visibility::Private(_) => {
                        Some(RustVisibility::Private)
                    }
                    pt::Visibility::External(_) => Some(RustVisibility::Public), // Treat external as public in this context
                }
            } else {
                None
            }
        })
        .unwrap_or(RustVisibility::Private); // Default to private if no visibility specified

    // Check for return statement and determine body type
    let body_contains_return = match &func.body {
        Some(statements) => statements_contains_return(&[statements.clone()]),
        None => false,
    };

    // Determine `is_view` and `is_endpoint`
    let is_view = func
        .name
        .as_ref()
        .map(|id| id.name.contains("get"))
        .unwrap_or(false);
    // TODO check for statement
    //&& body_contains_return;

    let is_endpoint = !is_view
        && !func
            .name
            .as_ref()
            .map(|id| id.name.is_empty())
            .unwrap_or(true);

    let body = match &func.body {
        Some(statements) => transform_statements(&[statements.clone()])?,
        None => Vec::new(),
    };

    Ok(RustNode::Function {
        name: func
            .name
            .as_ref()
            .map(|id| id.name.clone())
            .unwrap_or_default(),
        params,
        returns,
        body,
        visibility,
        is_endpoint,
        is_view,
    })
}

// Helper function to check for `return` statements in the body
fn statements_contains_return(statements: &[pt::Statement]) -> bool {
    statements
        .iter()
        .any(|stmt| matches!(stmt, pt::Statement::Return(_, _)))
}

fn transform_statements(statements: &[pt::Statement]) -> Result<Vec<RustNode>> {
    statements.iter().map(transform_statement).collect()
}

fn transform_statement(stmt: &pt::Statement) -> Result<RustNode> {
    match stmt {
        // Handle Block statements
        pt::Statement::Block { statements, .. } => {
            let body = statements
                .iter()
                .map(transform_statement)
                .collect::<Result<Vec<_>>>()?;
            Ok(RustNode::Function {
                name: "block".to_string(), // Placeholder for the block
                params: vec![],
                returns: None,
                body,
                visibility: RustVisibility::Private,
                is_endpoint: false,
                is_view: false,
            })
        }

        // Handle Assignment expressions
        pt::Statement::Expression(_, expr) => {
            match expr {
                // Match an assignment expression
                pt::Expression::Assign(_, left, right) => Ok(RustNode::Assignment {
                    target: Box::new(transform_expression(left)?),
                    value: Box::new(transform_expression(right)?),
                }),

                // Handle other expressions
                _ => {
                    println!("{:?}", expr);
                    Err(anyhow!("Unsupported expression: {:?}", expr))
                }
            }
        }

        // Handle Emit statements
        pt::Statement::Emit(_, function_call) => {
            if let pt::Expression::FunctionCall(_, function, args) = function_call {
                let event_name = match function.as_ref() {
                    pt::Expression::Variable(identifier) => identifier.name.clone(),
                    _ => return Err(anyhow!("Unsupported function in Emit")),
                };
                let transformed_args = args
                    .iter()
                    .map(transform_expression)
                    .collect::<Result<Vec<_>>>()?;
                Ok(RustNode::Expression(RustExpression::FunctionCall {
                    function: Box::new(RustExpression::Identifier(event_name)),
                    arguments: transformed_args,
                }))
            } else {
                Err(anyhow!("Unsupported Emit statement"))
            }
        }

        // Handle Return statements
        pt::Statement::Return(_, expr) => Ok(RustNode::Return(
            expr.as_ref().map(transform_expression).transpose()?,
        )),

        _ => Err(anyhow!("Unsupported statement type")),
    }
}
