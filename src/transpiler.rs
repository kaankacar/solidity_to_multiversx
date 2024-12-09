use crate::helper_functions::convert_expression_to_type;
use crate::parser::ParsedContract;
use crate::rust_ast::{RustExpression, RustNode, RustParameter, RustVisibility};
use crate::type_mapper::map_type;
use crate::statement::transform_statement;
use anyhow::{anyhow, Result};
use solang_parser::pt;

pub fn transform_with_attributes(parsed: ParsedContract) -> Result<String> {
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
            let rust_body = transform_contract_with_attributes(&contract)?;
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

                    let params_str = params
                        .iter()
                        .map(|p| format!("{}: {}", p.name, p.type_name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    // Add function signature
                    output.push_str(&format!(
                        "    {}fn {}(&self{}) {{\n",
                        function_visibility,
                        function_name,
                        if params_str.is_empty() {
                            String::new()
                        } else {
                            format!(", {}", params_str)
                        }
                    ));

                    // Add function body
                    println!("{:?} params", body);

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
        RustNode::Expression(RustExpression::FunctionCall {
            function,
            arguments,
        }) => {
            if let RustExpression::MemberAccess { expression, member } = *function.clone() {
                // Handle `increment` and `decrement` operations
                if member == "increment" || member == "decrement" {
                    let operation = if member == "increment" { "+" } else { "-" };

                    if let Some(arg) = arguments.first() {
                        let argument_code =
                            transform_rust_node_to_code(&RustNode::Expression(arg.clone()))?;

                        return Ok(format!(
                            "let current_value = self.{}().get();\n        self.{}().set(current_value {} 1);",
                            argument_code, argument_code, operation
                        ));
                    }
                // Handle `pre_increment` and `pre_decrement` operations
                } else if member == "pre_increment" || member == "pre_decrement" {
                    let operation = if member == "pre_increment" { "+" } else { "-" };

                    if let Some(arg) = arguments.first() {
                        let argument_code =
                            transform_rust_node_to_code(&RustNode::Expression(arg.clone()))?;

                        return Ok(format!(
                            "let current_value = self.{}().get() {} 1;\n        self.{}().set(current_value);",
                            argument_code, operation, argument_code
                        ));
                    }
                // Handle `set` calls on storage mappers
                } else if member == "set" {
                    if let Some(arg) = arguments.first() {
                        let storage_variable = transform_rust_node_to_code(&RustNode::Expression(
                            *expression.clone(),
                        ))?;
                        let argument_code =
                            transform_rust_node_to_code(&RustNode::Expression(arg.clone()))?;

                        return Ok(format!(
                            "self.{}().set({});",
                            storage_variable, argument_code
                        ));
                    }
                }
            }

            // Fallback to default function call handling for other cases
            let func_code = transform_rust_node_to_code(&RustNode::Expression(*function.clone()))?;
            let args_code = arguments
                .iter()
                .map(|arg| transform_rust_node_to_code(&RustNode::Expression(arg.clone())))
                .collect::<Result<Vec<_>>>()?
                .join(", ");
            Ok(format!("{}({})", func_code, args_code))
        }

        // Handle arithmetic operations
        RustNode::Expression(RustExpression::BinaryOperation {
            left,
            operator,
            right,
        }) => {
            if let RustExpression::FunctionCall {
                function,
                arguments,
            } = *left.clone()
            {
                
                if let RustExpression::MemberAccess { expression, member } = *function {
                    if member.ends_with(".get") {
                        let member_code =
                            transform_rust_node_to_code(&RustNode::Expression(*expression))?;
                        let right_code =
                            transform_rust_node_to_code(&RustNode::Expression(*right.clone()))?;

                        return Ok(format!(
                            "let current_value = self.{}().get();\n        self.{}().set(current_value {} {});",
                            member_code,
                            member_code,
                            operator,
                            right_code
                        ));
                    }
                }
            }

            // Fallback for other binary operations
            let left_code = transform_rust_node_to_code(&RustNode::Expression(*left.clone()))?;
            let right_code = transform_rust_node_to_code(&RustNode::Expression(*right.clone()))?;
            Ok(format!("{} {} {}", left_code, operator, right_code))
        }
        // Handle variable identifiers
        RustNode::Expression(RustExpression::Identifier(name)) => Ok(name.clone()),

        // Handle number literals
        RustNode::Expression(RustExpression::NumberLiteral(value)) => Ok(value.clone()),

        // Handle block nodes
        RustNode::Function { name, body, .. } if name == "block" => {
            let body_code = body
                .iter()
                .map(transform_rust_node_to_code)
                .collect::<Result<Vec<_>>>()?
                .join("\n        ");
            Ok(body_code)
        }

        // Handle assignments
        RustNode::Assignment { target, value } => {
            if let RustExpression::BinaryOperation {
                left,
                operator,
                right,
            } = *value.clone()
            {
                let operator_code = operator; // "+", "-", etc.
                let right_code = transform_rust_node_to_code(&RustNode::Expression(*right))?;
                let target_code =
                    transform_rust_node_to_code(&RustNode::Expression(*target.clone()))?;
                Ok(format!(
                    "let current_value = self.{}().get();\n        self.{}().set(current_value {} {});",
                    target_code, target_code, operator_code, right_code
                ))
            } else {
                Err(anyhow!("Unsupported assignment value: {:?}", value))
            }
        }

        // Handle function definitions
        RustNode::Function {
            name,
            params,
            body,
            visibility,
            ..
        } => {
            let visibility_str = if *visibility == RustVisibility::Public {
                "pub "
            } else {
                ""
            };

            let params_str = params
                .iter()
                .map(|param| format!("{}: {}", param.name, param.type_name))
                .collect::<Vec<_>>()
                .join(", ");

            let body_code = body
                .iter()
                .map(transform_rust_node_to_code)
                .collect::<Result<Vec<_>>>()?
                .join("\n        ");

            Ok(format!(
                "    {}fn {}(&self{}) {{\n        {}\n    }}",
                visibility_str,
                name,
                if params_str.is_empty() {
                    "".to_string()
                } else {
                    format!(", {}", params_str)
                },
                body_code
            ))
        }

        // Unsupported cases
        _ => Err(anyhow!("Unsupported RustNode type: {:?}", node)),
    }
}

fn transform_contract_with_attributes(contract: &pt::ContractDefinition) -> Result<Vec<RustNode>> {
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
                rust_nodes.push(RustNode::StorageDefinition { name, type_name });
            }
            pt::ContractPart::FunctionDefinition(func) => {
                rust_nodes.push(transform_function_with_attributes(func)?);
            }
            pt::ContractPart::EnumDefinition(enum_def) => {
                let name = enum_def
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .unwrap_or_default();
                let variants = enum_def
                    .values
                    .iter()
                    .map(|variant| {
                        variant
                            .as_ref()
                            .map(|id| id.name.clone())
                            .unwrap_or_default()
                    })
                    .collect();
                rust_nodes.push(RustNode::EnumDefinition { name, variants });
            }
            pt::ContractPart::StructDefinition(struct_def) => {
                let name = struct_def
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .unwrap_or_default();
                let fields = struct_def
                    .fields
                    .iter()
                    .map(|field| {
                        let field_name = field
                            .name
                            .as_ref()
                            .map(|id| id.name.clone())
                            .unwrap_or_default();
                        let field_type = map_type(&convert_expression_to_type(&field.ty)?)?;
                        Ok(RustParameter {
                            name: field_name,
                            type_name: field_type,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                rust_nodes.push(RustNode::StructDefinition { name, fields });
            }
            pt::ContractPart::EventDefinition(event_def) => {
                let name = event_def
                    .name
                    .as_ref()
                    .map(|id| id.name.clone())
                    .unwrap_or_default();
                let params = event_def
                    .fields
                    .iter()
                    .map(|param| {
                        let param_name = param
                            .name
                            .as_ref()
                            .map(|id| id.name.clone())
                            .unwrap_or_default();
                        let param_type = map_type(&convert_expression_to_type(&param.ty)?)?;
                        Ok(RustParameter {
                            name: param_name,
                            type_name: param_type,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                rust_nodes.push(RustNode::EventDefinition { name, params });
            }
            _ => {
                // Log unsupported parts for debugging
                println!("Unsupported contract part: {:?}", part);
            }
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
                    pt::Visibility::External(_) => Some(RustVisibility::Public),
                }
            } else {
                None
            }
        })
        .unwrap_or(RustVisibility::Private);

    // Check for return statement and determine body type
    let body_contains_return = match &func.body {
        Some(statements) => statements_contains_return(&[statements.clone()]),
        None => false,
    };

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
