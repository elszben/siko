use crate::constants;
use crate::location_info::location::Location;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum TypeSignature {
    Bool,
    Int,
    String,
    Nothing,
    Tuple(Vec<TypeSignature>),
    Function(Vec<TypeSignature>),
    Invalid(String, Location),
    TypeArgument(String, Location),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FunctionId {
    NamedFunctionId(NamedFunctionId),
    Lambda(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NamedFunctionId {
    pub module_name: String,
    pub function_name: String,
}

#[derive(Debug)]
pub enum Expr {
    Lambda(Vec<String>, FunctionId),
    FunctionCall(Box<Expr>, Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Tuple(Vec<Expr>),
    IntegerLiteral(i64, Location),
    FloatLiteral(f64, Location),
    BoolLiteral(bool, Location),
    StringLiteral(String, Location),
    Do(Vec<Expr>),
    Bind((String, Location), Box<Expr>),
    VariableRef(String, Location),
    FunctionRef(FunctionId, Location),
}

#[derive(Debug)]
pub enum FunctionBody {
    Expr(Expr),
    Extern,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub name: (String, Location),
    pub type_args: Vec<(String, Location)>,
    pub type_signature: TypeSignature,
}

#[derive(Debug)]
pub struct Function {
    pub lambda: bool,
    pub name: String,
    pub module: String,
    pub location: Location,
    pub args: Vec<(String, Location)>,
    pub body: FunctionBody,
    pub func_type: Option<FunctionType>,
}

impl Function {
    pub fn new(
        lambda: bool,
        name: String,
        module: String,
        location: Location,
        args: Vec<(String, Location)>,
        body: FunctionBody,
        func_type: Option<FunctionType>,
    ) -> Function {
        Function {
            lambda: lambda,
            name: name,
            module: module,
            location: location,
            args: args,
            body: body,
            func_type: func_type,
        }
    }

    pub fn get_function_id(&self) -> FunctionId {
        if self.lambda {
            FunctionId::Lambda(self.name.clone())
        } else {
            let named_id = NamedFunctionId {
                module_name: self.module.clone(),
                function_name: self.name.clone(),
            };
            FunctionId::NamedFunctionId(named_id)
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub location: Location,
    pub functions: BTreeMap<String, Function>,
}

impl Module {
    pub fn new(name: String, location: Location) -> Module {
        Module {
            name: name,
            location: location,
            functions: BTreeMap::new(),
        }
    }

    pub fn get_function(&self, name: &str) -> &Function {
        self.functions.get(name).expect("function not found")
    }
}

#[derive(Debug)]
pub struct Program {
    pub modules: BTreeMap<String, Module>,
    pub lambdas: BTreeMap<String, Function>,
}

impl Program {
    pub fn new() -> Program {
        Program {
            modules: BTreeMap::new(),
            lambdas: BTreeMap::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        if let Some(m) = self.modules.get(constants::MAIN_MODULE) {
            if let Some(f) = m.functions.get(constants::MAIN_FUNCTION) {
                return true;
            }
        }
        false
    }

    pub fn get_function(&self, module: &str, name: &str) -> &Function {
        let module = self.modules.get(module).expect("module not found");
        module.get_function(name)
    }

    pub fn get_lambda(&self, name: &str) -> &Function {
        self.lambdas.get(name).expect("lambda not found")
    }
}
