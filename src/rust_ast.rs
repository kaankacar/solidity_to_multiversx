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
    },
    EnumDefinition {
        name: String,
        variants: Vec<String>,
    },
    StructDefinition {
        name: String,
        fields: Vec<RustParameter>,
    },
    EventDefinition {
        name: String,
        params: Vec<RustParameter>,
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
    ForStatement {
        init: Option<Box<RustNode>>,
        condition: Option<Box<RustExpression>>,
        increment: Option<Box<RustNode>>,
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