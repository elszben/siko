use siko_constants::MIR_FUNCTION_TRAIT_NAME;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::data::ExternalDataKind;
use siko_mir::data::RecordKind;
use siko_mir::data::TypeDef;
use siko_mir::data::TypeDefId;
use siko_mir::expr::Expr;
use siko_mir::expr::ExprId;
use siko_mir::function::FunctionId;
use siko_mir::function::FunctionInfo;
use siko_mir::pattern::Pattern;
use siko_mir::pattern::PatternId;
use siko_mir::program::Program;
use siko_mir::types::Type;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::Result;
use std::io::Write;

struct Indent {
    indent: usize,
}

impl Indent {
    fn new() -> Indent {
        Indent { indent: 0 }
    }

    fn inc(&mut self) {
        self.indent += 4;
    }

    fn dec(&mut self) {
        self.indent -= 4;
    }
}

impl fmt::Display for Indent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for _ in 0..self.indent {
            write!(f, " ")?
        }
        Ok(())
    }
}

fn get_module_name(name: &str) -> String {
    name.replace(".", "_")
}

fn ir_type_to_rust_type(ty: &Type, program: &Program) -> String {
    fn ir_type_to_rust_type_inner(ty: &Type, program: &Program) -> String {
        match ty {
            Type::Function(from, to) => {
                let from = ir_type_to_rust_type_inner(from, program);
                let to = ir_type_to_rust_type_inner(to, program);
                format!(
                    "::crate::{}::{}<{}, {}>",
                    MIR_INTERNAL_MODULE_NAME, MIR_FUNCTION_TRAIT_NAME, from, to
                )
            }
            Type::Named(id) => {
                let typedef = program.typedefs.get(id);
                let (module_name, name) = match typedef {
                    TypeDef::Adt(adt) => (get_module_name(&adt.module), adt.name.clone()),
                    TypeDef::Record(record) => {
                        (get_module_name(&record.module), record.name.clone())
                    }
                };
                format!("crate::{}::{}", module_name, name)
            }
        }
    }

    match ty {
        Type::Function(..) => {
            let s = ir_type_to_rust_type_inner(ty, program);
            format!("Box<{}>", s)
        }
        Type::Named(..) => ir_type_to_rust_type_inner(ty, program),
    }
}

fn write_typedef(
    typedef_id: TypeDefId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<()> {
    let typedef = program.typedefs.get(&typedef_id);
    match typedef {
        TypeDef::Adt(adt) => {
            write!(output_file, "{}pub enum {} {{\n", indent, adt.name)?;
            indent.inc();
            for variant in &adt.variants {
                let items = if variant.items.is_empty() {
                    format!("")
                } else {
                    let mut is = Vec::new();
                    for item in &variant.items {
                        let rust_ty = ir_type_to_rust_type(&item, program);
                        is.push(rust_ty);
                    }
                    format!("({})", is.join(", "))
                };
                write!(output_file, "{}{}{},\n", indent, variant.name, items)?;
            }
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
        }
        TypeDef::Record(record) => {
            if let RecordKind::External(data_kind, args) = &record.kind {
                match data_kind {
                    ExternalDataKind::Int => {
                        write!(output_file, "{}pub struct Int {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: i64,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::String => {
                        write!(output_file, "{}pub struct String {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: std::string::String,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Float => {
                        write!(output_file, "{}pub struct Float {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: f64,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Map => {
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Iterator => {
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::List => {
                        let elem_ty = ir_type_to_rust_type(&args[0], program);
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        indent.inc();
                        write!(output_file, "{}pub value: Vec<{}>,\n", indent, elem_ty)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                }
            } else {
                write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                indent.inc();
                for field in &record.fields {
                    let field_type = ir_type_to_rust_type(&field.ty, program);
                    write!(
                        output_file,
                        "{}pub {}: {},\n",
                        indent, field.name, field_type
                    )?;
                }
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
            }
        }
    }
    Ok(())
}

fn arg_name(index: usize) -> String {
    format!("arg{}", index)
}

fn write_pattern(
    pattern_id: PatternId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<()> {
    let pattern = &program.patterns.get(&pattern_id).item;
    match pattern {
        Pattern::Binding(name) => {
            write!(output_file, "{}", name)?;
        }
        Pattern::Record(id, items) => {
            let ty = program.get_pattern_type(&pattern_id);
            let record = program.typedefs.get(id).get_record();
            write!(output_file, "{} {{", ir_type_to_rust_type(ty, program))?;
            for (index, item) in items.iter().enumerate() {
                let field = &record.fields[index];
                write!(output_file, "{}: ", field.name)?;
                write_pattern(*item, output_file, program, indent)?;
                write!(output_file, ", ")?;
            }
            write!(output_file, "}}")?;
        }
        Pattern::Variant(id, index, items) => {
            let ty = program.get_pattern_type(&pattern_id);
            let adt = program.typedefs.get(id).get_adt();
            let variant = &adt.variants[*index];
            write!(
                output_file,
                "{}::{}",
                ir_type_to_rust_type(ty, program),
                variant.name
            )?;
            if !items.is_empty() {
                write!(output_file, "(")?;
                for (index, item) in items.iter().enumerate() {
                    write_pattern(*item, output_file, program, indent)?;
                    if index != items.len() - 1 {
                        write!(output_file, ", ")?;
                    }
                }
                write!(output_file, ")")?;
            }
        }
        Pattern::Guarded(pattern, expr) => {
            write_pattern(*pattern, output_file, program, indent)?;
            write!(output_file, " if ")?;
            write_expr(*expr, output_file, program, indent)?;
        }
        Pattern::Wildcard => {
            write!(output_file, "_")?;
        }
        Pattern::IntegerLiteral(i) => {
            write!(output_file, "{}", i)?;
        }
        Pattern::StringLiteral(s) => {
            write!(output_file, "{}", s)?;
        }
    }
    Ok(())
}

fn write_expr(
    expr_id: ExprId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<()> {
    let expr = &program.exprs.get(&expr_id).item;
    match expr {
        Expr::ArgRef(i) => {
            let arg = arg_name(*i);
            write!(output_file, "{}", arg)?;
        }
        Expr::Do(items) => {
            write!(output_file, "{{\n")?;
            indent.inc();
            for (index, item) in items.iter().enumerate() {
                write!(output_file, "{}", indent)?;
                write_expr(*item, output_file, program, indent)?;
                if index != items.len() - 1 {
                    write!(output_file, ";\n")?;
                }
            }
            indent.dec();
            write!(output_file, "\n{}}}", indent)?;
        }
        Expr::RecordInitialization(id, items) => {
            let ty = program.get_expr_type(&expr_id);
            let record = program.typedefs.get(id).get_record();
            write!(output_file, "{} {{", ir_type_to_rust_type(ty, program))?;
            for (item, index) in items {
                let field = &record.fields[*index];
                write!(output_file, "{}: ", field.name)?;
                write_expr(*item, output_file, program, indent)?;
                write!(output_file, ", ")?;
            }
            write!(output_file, "}}")?;
        }
        Expr::RecordUpdate(receiver, items) => {
            let ty = program.get_expr_type(&expr_id);
            let id = ty.get_typedef_id();
            let record = program.typedefs.get(&id).get_record();
            write!(output_file, "{{ let mut value = ")?;
            indent.inc();
            write_expr(*receiver, output_file, program, indent)?;
            write!(output_file, ";\n")?;
            for (item, index) in items {
                let field = &record.fields[*index];
                write!(output_file, "{}value.{} = ", indent, field.name)?;
                write_expr(*item, output_file, program, indent)?;
                write!(output_file, ";\n")?;
            }
            write!(output_file, "{}value }}", indent)?;
            indent.dec();
        }
        Expr::Bind(pattern, rhs) => {
            write!(output_file, "let ")?;
            write_pattern(*pattern, output_file, program, indent)?;
            write!(output_file, " = ")?;
            write_expr(*rhs, output_file, program, indent)?;
        }
        Expr::ExprValue(_, pattern_id) => {
            let pattern = &program.patterns.get(pattern_id).item;
            if let Pattern::Binding(n) = pattern {
                write!(output_file, "{}", n)?;
            } else {
                unreachable!();
            }
        }
        Expr::StaticFunctionCall(id, args) => {
            let function = program.functions.get(id);
            let name = format!(
                "crate::{}::{}",
                get_module_name(&function.module),
                function.name
            );
            write!(output_file, "{} (", name)?;
            for (index, arg) in args.iter().enumerate() {
                write_expr(*arg, output_file, program, indent)?;
                if index != args.len() - 1 {
                    write!(output_file, ", ")?;
                }
            }
            write!(output_file, ")")?;
        }
        Expr::IntegerLiteral(i) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {} }}", ty, i)?;
        }
        Expr::StringLiteral(s) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: format!(\"{}\") }}", ty, s)?;
        }
        Expr::FloatLiteral(f) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {:.5} }}", ty, f)?;
        }
        Expr::Formatter(fmt, args) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value : format!(\"{}\"", ty, fmt)?;
            if !args.is_empty() {
                write!(output_file, ",")?;
            }
            for (index, arg) in args.iter().enumerate() {
                write_expr(*arg, output_file, program, indent)?;
                write!(output_file, ".value")?;
                if index != args.len() - 1 {
                    write!(output_file, ",")?;
                }
            }
            write!(output_file, ")}}")?;
        }
        Expr::CaseOf(body, cases) => {
            write!(output_file, "match ")?;
            write_expr(*body, output_file, program, indent)?;
            write!(output_file, " {{\n")?;
            indent.inc();
            for case in cases {
                write!(output_file, "{}", indent)?;
                write_pattern(case.pattern_id, output_file, program, indent)?;
                write!(output_file, " => {{")?;
                write_expr(case.body, output_file, program, indent)?;
                write!(output_file, "}}\n")?;
            }
            indent.dec();
            write!(output_file, "{}}}", indent)?;
        }
        Expr::If(cond, true_branch, false_branch) => {
            write!(output_file, "if ")?;
            write_expr(*cond, output_file, program, indent)?;
            write!(output_file, " {{ ")?;
            write_expr(*true_branch, output_file, program, indent)?;
            write!(output_file, " }} ")?;
            write!(output_file, " else {{ ")?;
            write_expr(*false_branch, output_file, program, indent)?;
            write!(output_file, " }} ")?;
        }
        Expr::FieldAccess(index, receiver) => {
            let ty = program.get_expr_type(receiver);
            let id = ty.get_typedef_id();
            let record = program.typedefs.get(&id).get_record();
            let field = &record.fields[*index];
            write_expr(*receiver, output_file, program, indent)?;
            write!(output_file, ".{}", field.name)?;
        }
        Expr::List(items) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: vec![", ty)?;
            for (index, item) in items.iter().enumerate() {
                write_expr(*item, output_file, program, indent)?;
                if index != items.len() - 1 {
                    write!(output_file, ", ")?;
                }
            }
            write!(output_file, "] }}")?;
        }
        _ => println!("{:?}", expr),
    }
    Ok(())
}

fn write_function(
    function_id: FunctionId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<()> {
    let function = program.functions.get(&function_id);
    let mut fn_args = Vec::new();
    function.function_type.get_args(&mut fn_args);
    let mut args: Vec<String> = Vec::new();
    for i in 0..function.arg_count {
        let arg_ty = ir_type_to_rust_type(&fn_args[i], program);
        let arg_str = format!("{}: {}", arg_name(i), arg_ty);
        args.push(arg_str);
    }
    let args: String = args.join(", ");
    let result_ty = function.function_type.get_result_type(function.arg_count);
    let result_ty = ir_type_to_rust_type(&result_ty, program);
    write!(
        output_file,
        "{}pub fn {}({}) -> {} {{\n",
        indent, function.name, args, result_ty
    )?;
    match &function.info {
        FunctionInfo::Normal(body) => {
            indent.inc();
            write!(output_file, "{}", indent)?;
            write_expr(*body, output_file, program, indent)?;
            indent.dec();
        }
        FunctionInfo::Extern(original_name) => {
            indent.inc();
            match (function.module.as_ref(), original_name.as_ref()) {
                ("Int", "opAdd") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value + arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "opSub") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value - arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "opMul") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value * arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "opDiv") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value / arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "opEq") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value == arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("String", "opEq") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value == arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "show") => {
                    write!(
                        output_file,
                        "{}let value = format!(\"{{}}\", arg0.value);\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "partialCmp") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Int", "cmp") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("String", "partialCmp") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("String", "opAdd") => {
                    write!(
                        output_file,
                        "{}let value = format!(\"{{}}{{}}\", arg0.value, arg1.value);\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("String", "cmp") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "show") => {
                    write!(
                        output_file,
                        "{}let value = format!(\"{{}}\", arg0.value);\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "partialCmp") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "opAdd") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value + arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "opSub") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value - arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "opMul") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value * arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "opDiv") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value / arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Float", "opEq") => {
                    write!(
                        output_file,
                        "{}let value = arg0.value == arg1.value;\n",
                        indent
                    )?;
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Std.Ops", "opAnd") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Std.Ops", "opOr") => {
                    write!(output_file, "{}{} {{ value : value }}", indent, result_ty)?;
                }
                ("Std.Util.Basic", "println") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Std.Util", "assert") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Map", "empty") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Map", "insert") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Map", "remove") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Map", "get") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Iterator", "map") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("Iterator", "filter") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("List", "toList") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("List", "iter") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("List", "opEq") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                ("List", "show") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty)?;
                }
                _ => panic!("{}/{} not implemented", function.module, function.name),
            }
            indent.dec();
        }
        FunctionInfo::VariantConstructor(id, index) => {
            let adt = program.typedefs.get(id).get_adt();
            let variant = &adt.variants[*index];
            indent.inc();
            write!(output_file, "{}{}::{}", indent, result_ty, variant.name)?;
            if function.arg_count > 0 {
                let mut args = Vec::new();
                for i in 0..function.arg_count {
                    let arg_str = format!("{}", arg_name(i));
                    args.push(arg_str);
                }
                write!(output_file, "({})", args.join(", "))?;
            }
            indent.dec();
        }
        FunctionInfo::RecordConstructor(id) => {
            let record = program.typedefs.get(id).get_record();
            indent.inc();
            write!(output_file, "{}{}", indent, result_ty)?;
            let mut args = Vec::new();
            for (index, field) in record.fields.iter().enumerate() {
                let arg_str = format!("{}: {}", field.name, arg_name(index));
                args.push(arg_str);
            }
            write!(output_file, "{{ {} }}", args.join(", "))?;
            indent.dec();
        }
    }
    write!(output_file, "\n{}}}\n", indent,)?;
    Ok(())
}

struct Module {
    name: String,
    functions: Vec<FunctionId>,
    typedefs: Vec<TypeDefId>,
}

impl Module {
    fn new(name: String) -> Module {
        Module {
            name: name,
            functions: Vec::new(),
            typedefs: Vec::new(),
        }
    }

    fn write(
        &self,
        output_file: &mut dyn Write,
        program: &Program,
        indent: &mut Indent,
    ) -> Result<()> {
        write!(output_file, "mod {} {{\n", get_module_name(&self.name))?;
        indent.inc();
        for typedef_id in &self.typedefs {
            write_typedef(*typedef_id, output_file, program, indent)?;
        }
        for function_id in &self.functions {
            write_function(*function_id, output_file, program, indent)?;
        }
        indent.dec();
        write!(output_file, "}}\n\n",)?;
        Ok(())
    }
}

struct RustProgram {
    modules: BTreeMap<String, Module>,
}

impl RustProgram {
    fn new() -> RustProgram {
        RustProgram {
            modules: BTreeMap::new(),
        }
    }

    fn get_module(&mut self, module_name: String) -> &mut Module {
        let module = self
            .modules
            .entry(module_name.clone())
            .or_insert_with(|| Module::new(module_name.clone()));
        module
    }

    fn write(&self, output_file: &mut dyn Write, program: &Program) -> Result<()> {
        let mut indent = Indent::new();
        for (_, module) in &self.modules {
            module.write(output_file, program, &mut indent)?;
        }
        write!(output_file, "fn main() {{\n")?;
        indent.inc();
        write!(output_file, "{}crate::Main::main_0();\n", indent)?;
        write!(output_file, "}}\n")?;
        Ok(())
    }
}

pub struct Transpiler {}

impl Transpiler {
    pub fn process(program: &Program, target_file: &str) -> Result<()> {
        let filename = format!("{}", target_file);
        println!("Transpiling to {}", filename);
        let mut output_file = File::create(filename)?;
        write!(output_file, "#![allow(non_snake_case)]\n")?;
        write!(output_file, "#![allow(non_camel_case_types)]\n")?;
        write!(output_file, "#![allow(unused_variables)]\n")?;
        write!(output_file, "#![allow(dead_code)]\n\n")?;
        let mut rust_program = RustProgram::new();
        for (id, function) in program.functions.items.iter() {
            let module = rust_program.get_module(function.module.clone());
            module.functions.push(*id);
        }
        for (id, typedef) in program.typedefs.items.iter() {
            match typedef {
                TypeDef::Adt(adt) => {
                    let module = rust_program.get_module(adt.module.clone());
                    module.typedefs.push(*id);
                }
                TypeDef::Record(record) => {
                    let module = rust_program.get_module(record.module.clone());
                    module.typedefs.push(*id);
                }
            }
        }
        rust_program.write(&mut output_file, program)?;
        Ok(())
    }
}
