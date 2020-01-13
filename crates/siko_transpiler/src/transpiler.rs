use siko_constants::MIR_FUNCTION_TRAIT_NAME;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::data::ExternalDataKind;
use siko_mir::data::RecordKind;
use siko_mir::data::TypeDef;
use siko_mir::data::TypeDefId;
use siko_mir::expr::Expr;
use siko_mir::expr::ExprId;
use siko_mir::function::Function;
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
    match ty {
        Type::Function(from, to) => {
            let from = ir_type_to_rust_type(from, program);
            let to = ir_type_to_rust_type(to, program);
            format!(
                "Box<dyn ::crate::{}::{}<{}, {}>>",
                MIR_INTERNAL_MODULE_NAME, MIR_FUNCTION_TRAIT_NAME, from, to
            )
        }
        Type::Named(id) => {
            let typedef = program.typedefs.get(id);
            let (module_name, name) = match typedef {
                TypeDef::Adt(adt) => (get_module_name(&adt.module), adt.name.clone()),
                TypeDef::Record(record) => (get_module_name(&record.module), record.name.clone()),
            };
            format!("crate::{}::{}", module_name, name)
        }
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
            write!(output_file, "{}#[derive(Clone)]\n", indent)?;
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
                        write!(
                            output_file,
                            "{}#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\n",
                            indent
                        )?;
                        write!(output_file, "{}pub struct Int {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: i64,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::String => {
                        write!(
                            output_file,
                            "{}#[derive(Clone, PartialEq, Eq, PartialOrd)]\n",
                            indent
                        )?;
                        write!(output_file, "{}pub struct String {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: std::string::String,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Float => {
                        write!(
                            output_file,
                            "{}#[derive(Clone, PartialEq, PartialOrd)]\n",
                            indent
                        )?;
                        write!(output_file, "{}pub struct Float {{\n", indent)?;
                        indent.inc();
                        write!(output_file, "{}pub value: f64,\n", indent,)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Map => {
                        write!(
                            output_file,
                            "{}#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]\n",
                            indent
                        )?;
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        indent.inc();
                        let key_ty = ir_type_to_rust_type(&args[0], program);
                        let value_ty = ir_type_to_rust_type(&args[1], program);
                        write!(
                            output_file,
                            "{}pub value: std::collections::BTreeMap<{}, {}>,\n",
                            indent, key_ty, value_ty
                        )?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::Iterator => {
                        write!(output_file, "{}#[derive(Clone)]\n", indent)?;
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        write!(output_file, "{}}}\n", indent)?;
                    }
                    ExternalDataKind::List => {
                        let elem_ty = ir_type_to_rust_type(&args[0], program);
                        write!(output_file, "{}#[derive(Clone)]\n", indent)?;
                        write!(output_file, "{}pub struct {} {{\n", indent, record.name)?;
                        indent.inc();
                        write!(output_file, "{}pub value: Vec<{}>,\n", indent, elem_ty)?;
                        indent.dec();
                        write!(output_file, "{}}}\n", indent)?;
                    }
                }
            } else {
                write!(output_file, "{}#[derive(Clone)]\n", indent)?;
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
            write!(output_file, " if {{ match ")?;
            write_expr(*expr, output_file, program, indent)?;
            let ty = program.get_expr_type(expr);
            let ty = ir_type_to_rust_type(ty, program);
            write!(
                output_file,
                "  {{ {}::True => true, {}::False => false }} }}",
                ty, ty
            )?;
        }
        Pattern::Wildcard => {
            write!(output_file, "_")?;
        }
        Pattern::IntegerLiteral(i) => {
            let ty = program.get_pattern_type(&pattern_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {} }}", ty, i)?;
        }
        Pattern::StringLiteral(s) => {
            let ty = program.get_pattern_type(&pattern_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {} }}", ty, s)?;
        }
    }
    Ok(())
}

fn write_expr(
    expr_id: ExprId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<bool> {
    let mut is_statement = false;
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
                let is_statement = write_expr(*item, output_file, program, indent)?;
                if is_statement {
                    if index == items.len() - 1 {
                        let ty = program.get_expr_type(&expr_id);
                        write!(output_file, "{} {{ }} ", ir_type_to_rust_type(ty, program))?;
                    } else {
                        write!(output_file, "\n")?;
                    }
                } else {
                    if index != items.len() - 1 {
                        write!(output_file, ";\n")?;
                    }
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
            write!(output_file, ";")?;
            is_statement = true;
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
            if function.arg_count != args.len() {
                write!(output_file, "Box::new(ClosureData{}{{", expr_id.id)?;
                for (index, arg) in args.iter().enumerate() {
                    write!(output_file, "arg{} : ", index)?;
                    write_expr(*arg, output_file, program, indent)?;
                    if index != args.len() - 1 {
                        write!(output_file, ", ")?;
                    }
                }
                write!(output_file, "}})")?;
            } else {
                let name = format!(
                    "crate::{}::{}",
                    get_module_name(&function.module),
                    function.name
                );
                write!(output_file, "{} (", name)?;
                for (index, arg) in args.iter().enumerate() {
                    write_expr(*arg, output_file, program, indent)?;
                    write!(output_file, ".clone()")?;
                    if index != args.len() - 1 {
                        write!(output_file, ", ")?;
                    }
                }
                write!(output_file, ")")?;
            }
        }
        Expr::IntegerLiteral(i) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {} }}", ty, i)?;
        }
        Expr::StringLiteral(s) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: \"{}\".to_string() }}", ty, s)?;
        }
        Expr::FloatLiteral(f) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: {:.5} }}", ty, f)?;
        }
        Expr::CharLiteral(c) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: '{}' }}", ty, c)?;
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
            write!(output_file, "match (")?;
            write_expr(*body, output_file, program, indent)?;
            write!(output_file, ") {{\n")?;
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
            let ty = program.get_expr_type(cond);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "if {{ match (")?;
            write_expr(*cond, output_file, program, indent)?;
            write!(
                output_file,
                ") {{ {}::True => true, {}::False => false, }} }} ",
                ty, ty
            )?;
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
        Expr::DynamicFunctionCall(receiver, args) => {
            indent.inc();
            write!(output_file, "{{{}let mut dyn_fn = ", indent)?;
            write_expr(*receiver, output_file, program, indent)?;
            write!(output_file, ";\n")?;
            for arg in args {
                write!(output_file, "{}dyn_fn = dyn_fn.call(", indent)?;
                write_expr(*arg, output_file, program, indent)?;
                write!(output_file, ");\n")?;
            }
            write!(output_file, "{}dyn_fn }}", indent)?;
            indent.dec();
        }
    }
    Ok(is_statement)
}

fn generate_partial_cmp_builtin_body(
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    result_ty: &Type,
    result_ty_str: &str,
) -> Result<()> {
    let id = result_ty.get_typedef_id();
    let adt_opt = program.typedefs.get(&id).get_adt();
    let mut ord_ty = None;
    for (index, v) in adt_opt.variants.iter().enumerate() {
        if v.name == "Some" {
            ord_ty = Some(v.items[0].clone());
        }
    }
    let ord_ty = ord_ty.expect("Ord ty not found");
    let ord_ty_str = ir_type_to_rust_type(&ord_ty, program);
    write!(output_file, "{}match arg0.partial_cmp(&arg1) {{\n", indent)?;
    indent.inc();
    write!(
        output_file,
        "{} Some(std::cmp::Ordering::Less) => {{ {}::Some({}::Less) }}\n",
        indent, result_ty_str, ord_ty_str
    )?;
    write!(
        output_file,
        "{} Some(std::cmp::Ordering::Equal) => {{ {}::Some({}::Equal) }}\n",
        indent, result_ty_str, ord_ty_str
    )?;
    write!(
        output_file,
        "{} Some(std::cmp::Ordering::Greater) => {{ {}::Some({}::Greater) }}\n",
        indent, result_ty_str, ord_ty_str
    )?;
    write!(
        output_file,
        "{} None => {{ {}::None }}\n",
        indent, result_ty_str
    )?;
    indent.dec();
    write!(output_file, "{}}}", indent)?;

    Ok(())
}

fn generate_cmp_builtin_body(
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(output_file, "{}match arg0.cmp(&arg1) {{\n", indent)?;
    indent.inc();
    write!(
        output_file,
        "{} std::cmp::Ordering::Less => {{ {}::Less }}\n",
        indent, result_ty_str
    )?;
    write!(
        output_file,
        "{} std::cmp::Ordering::Equal => {{ {}::Equal }}\n",
        indent, result_ty_str
    )?;
    write!(
        output_file,
        "{} std::cmp::Ordering::Greater => {{ {}::Greater }}\n",
        indent, result_ty_str
    )?;
    indent.dec();
    write!(output_file, "{}}}", indent)?;

    Ok(())
}

fn generate_show_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = format!(\"{{}}\", arg0.value);\n",
        indent
    )?;
    write!(
        output_file,
        "{}{} {{ value : value }}",
        indent, result_ty_str
    )?;
    Ok(())
}

fn generate_opeq_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = arg0.value == arg1.value;\n",
        indent
    )?;
    write!(
        output_file,
        "{} match value {{ true => {}::True, false => {}::False, }}",
        indent, result_ty_str, result_ty_str
    )?;
    Ok(())
}

fn generate_opdiv_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = arg0.value / arg1.value;\n",
        indent
    )?;
    write!(
        output_file,
        "{}{} {{ value : value }}",
        indent, result_ty_str
    )?;
    Ok(())
}

fn generate_opmul_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = arg0.value * arg1.value;\n",
        indent
    )?;
    write!(
        output_file,
        "{}{} {{ value : value }}",
        indent, result_ty_str
    )?;
    Ok(())
}

fn generate_opsub_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = arg0.value - arg1.value;\n",
        indent
    )?;
    write!(
        output_file,
        "{}{} {{ value : value }}",
        indent, result_ty_str
    )?;
    Ok(())
}

fn generate_opadd_builtin_body(
    output_file: &mut dyn Write,
    indent: &mut Indent,
    result_ty_str: &str,
) -> Result<()> {
    write!(
        output_file,
        "{}let value = arg0.value + arg1.value;\n",
        indent
    )?;
    write!(
        output_file,
        "{}{} {{ value : value }}",
        indent, result_ty_str
    )?;
    Ok(())
}

fn generate_num_builtins(
    module: &str,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    original_name: &str,
    result_ty: &Type,
    result_ty_str: &str,
) -> Result<()> {
    indent.inc();
    match original_name {
        "opAdd" => {
            generate_opadd_builtin_body(output_file, indent, result_ty_str)?;
        }
        "opSub" => {
            generate_opsub_builtin_body(output_file, indent, result_ty_str)?;
        }
        "opMul" => {
            generate_opmul_builtin_body(output_file, indent, result_ty_str)?;
        }
        "opDiv" => {
            generate_opdiv_builtin_body(output_file, indent, result_ty_str)?;
        }
        "opEq" => {
            generate_opeq_builtin_body(output_file, indent, result_ty_str)?;
        }
        "show" => {
            generate_show_builtin_body(output_file, indent, result_ty_str)?;
        }
        "partialCmp" => {
            generate_partial_cmp_builtin_body(
                output_file,
                program,
                indent,
                result_ty,
                result_ty_str,
            )?;
        }
        "cmp" => {
            generate_cmp_builtin_body(output_file, program, indent, result_ty_str)?;
        }
        _ => panic!("{}/{} not implemented", module, original_name),
    }
    indent.dec();
    Ok(())
}

fn generate_string_builtins(
    module: &str,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    original_name: &str,
    result_ty: &Type,
    result_ty_str: &str,
) -> Result<()> {
    indent.inc();
    match original_name {
        "opAdd" => {
            write!(
                output_file,
                "{}let value = format!(\"{{}}{{}}\", arg0.value, arg1.value);\n",
                indent
            )?;
            write!(
                output_file,
                "{}{} {{ value : value }}",
                indent, result_ty_str
            )?;
        }
        "opEq" => {
            generate_opeq_builtin_body(output_file, indent, result_ty_str)?;
        }
        "partialCmp" => {
            generate_partial_cmp_builtin_body(
                output_file,
                program,
                indent,
                result_ty,
                result_ty_str,
            )?;
        }
        "cmp" => {
            generate_cmp_builtin_body(output_file, program, indent, result_ty_str)?;
        }
        _ => panic!("{}/{} not implemented", module, original_name),
    }
    indent.dec();
    Ok(())
}

fn generate_map_builtins(
    function: &Function,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    original_name: &str,
    result_ty: &Type,
    result_ty_str: &str,
) -> Result<()> {
    indent.inc();
    match original_name {
        "empty" => {
            write!(
                output_file,
                "{}let value = std::collections::BTreeMap::new();\n",
                indent
            )?;
            write!(
                output_file,
                "{}{} {{ value : value }}",
                indent, result_ty_str
            )?;
        }
        "insert" => {
            let result_id = result_ty.get_typedef_id();
            let tuple_record = program.typedefs.get(&result_id).get_record();
            let option_ty = ir_type_to_rust_type(&tuple_record.fields[1].ty, program);
            write!(output_file, "{}let mut arg0 = arg0;\n", indent)?;
            write!(
                output_file,
                "{}let value = match arg0.value.insert(arg1, arg2) {{\n",
                indent
            )?;
            indent.inc();
            write!(
                output_file,
                "{} Some(v) => {}::Some(v),\n",
                indent, option_ty
            )?;
            write!(output_file, "{} None => {}::None,\n", indent, option_ty)?;
            indent.dec();
            write!(output_file, "{}}};\n", indent)?;
            write!(
                output_file,
                "{}{} {{ field_0 : arg0, field_1: value }}",
                indent, result_ty_str
            )?;
        }
        "remove" => {
            let result_id = result_ty.get_typedef_id();
            let tuple_record = program.typedefs.get(&result_id).get_record();
            let option_ty = ir_type_to_rust_type(&tuple_record.fields[1].ty, program);
            write!(output_file, "{}let mut arg0 = arg0;\n", indent)?;
            write!(
                output_file,
                "{}let value = match arg0.value.remove(&arg1) {{\n",
                indent
            )?;
            indent.inc();
            write!(
                output_file,
                "{} Some(v) => {}::Some(v),\n",
                indent, option_ty
            )?;
            write!(output_file, "{} None => {}::None,\n", indent, option_ty)?;
            indent.dec();
            write!(output_file, "{}}};\n", indent)?;
            write!(
                output_file,
                "{}{} {{ field_0 : arg0, field_1: value }}",
                indent, result_ty_str
            )?;
        }
        "get" => {
            write!(output_file, "{}match arg0.value.get(&arg1) {{\n", indent)?;
            indent.inc();
            write!(
                output_file,
                "{} Some(v) => {}::Some(v.clone()),\n",
                indent, result_ty_str
            )?;
            write!(output_file, "{} None => {}::None,\n", indent, result_ty_str)?;
            indent.dec();
            write!(output_file, "{}}}", indent)?;
        }
        _ => panic!("Map/{} not implemented", original_name),
    }
    indent.dec();
    Ok(())
}

fn generate_list_builtins(
    function: &Function,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    original_name: &str,
    result_ty_str: &str,
) -> Result<()> {
    indent.inc();
    match original_name {
        "show" => {
            write!(
                output_file,
                "{}let subs: Vec<_> = arg0.value.iter().map(|item| format!(\"{{}}\", item)).collect();\n",
                indent
            )?;
            write!(
                output_file,
                "{}{} {{ value : format!(\"[{{}}]\", subs.join(\", \")) }}",
                indent, result_ty_str
            )?;
        }
        "toList" => {}
        "iter" => {}
        "opEq" => {}
        _ => panic!("List/{} not implemented", original_name),
    }
    indent.dec();
    Ok(())
}

fn generate_builtin(
    function: &Function,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    original_name: &str,
    result_ty: &Type,
    result_ty_str: &str,
    arg_types: Vec<String>,
) -> Result<()> {
    match function.module.as_ref() {
        "Int" => {
            return generate_num_builtins(
                function.module.as_ref(),
                output_file,
                program,
                indent,
                original_name,
                result_ty,
                result_ty_str,
            );
        }
        "String" => {
            return generate_string_builtins(
                function.module.as_ref(),
                output_file,
                program,
                indent,
                original_name,
                result_ty,
                result_ty_str,
            );
        }
        "Float" => {
            return generate_num_builtins(
                function.module.as_ref(),
                output_file,
                program,
                indent,
                original_name,
                result_ty,
                result_ty_str,
            );
        }
        "Map" => {
            return generate_map_builtins(
                function,
                output_file,
                program,
                indent,
                original_name,
                result_ty,
                result_ty_str,
            );
        }
        "List" => {
            return generate_list_builtins(
                function,
                output_file,
                program,
                indent,
                original_name,
                result_ty_str,
            );
        }
        _ => {
            indent.inc();
            match (function.module.as_ref(), original_name) {
                ("Std.Ops", "opAnd") => {
                    write!(
                        output_file,
                        "{} match (arg0, arg1) {{ ({}::True, {}::True) => {}::True,",
                        indent, result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::True, {}::False) => {}::False,",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::False, {}::True) => {}::False,",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::False, {}::False) => {}::False, }}",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                }
                ("Std.Ops", "opOr") => {
                    write!(
                        output_file,
                        "{} match (arg0, arg1) {{ ({}::True, {}::True) => {}::True,",
                        indent, result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::True, {}::False) => {}::True,",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::False, {}::True) => {}::True,",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                    write!(
                        output_file,
                        "({}::False, {}::False) => {}::False, }}",
                        result_ty_str, result_ty_str, result_ty_str,
                    )?;
                }
                ("Std.Util.Basic", "println") => {
                    write!(output_file, "{}println!(\"{{}}\", arg0);\n", indent)?;
                    write!(output_file, "{}{} {{ }}", indent, result_ty_str)?;
                }
                ("Std.Util.Basic", "print") => {
                    write!(output_file, "{}print!(\"{{}}\", arg0);\n", indent)?;
                    write!(output_file, "{}{} {{ }}", indent, result_ty_str)?;
                }
                ("Std.Util", "assert") => {
                    let panic = "{{ panic!(\"Assertion failed\"); }}";
                    write!(
                        output_file,
                        "{} match arg0 {{ {}::True => {{}}, {}::False => {} }}",
                        indent, arg_types[0], arg_types[0], panic
                    )?;
                    write!(output_file, "{}{} {{ }}", indent, result_ty_str)?;
                }
                ("Iterator", "map") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty_str)?;
                }
                ("Iterator", "filter") => {
                    write!(output_file, "{}{} {{ }}", indent, result_ty_str)?;
                }
                _ => panic!("{}/{} not implemented", function.module, function.name),
            }
        }
    }
    indent.dec();
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
    let mut arg_types: Vec<String> = Vec::new();
    for i in 0..function.arg_count {
        let arg_ty = ir_type_to_rust_type(&fn_args[i], program);
        let arg_str = format!("{}: {}", arg_name(i), arg_ty);
        arg_types.push(arg_ty);
        args.push(arg_str);
    }
    let args: String = args.join(", ");
    let result_type = function.function_type.get_result_type(function.arg_count);
    let result_ty = ir_type_to_rust_type(&result_type, program);
    if let FunctionInfo::ExternClassImpl(class_name, ty, body) = &function.info {
        if class_name == "show" {
            let impl_ty = ir_type_to_rust_type(&ty, program);
            write!(
                output_file,
                "{}impl std::fmt::Display for {} {{\n",
                indent, impl_ty,
            )?;
            indent.inc();
            write!(
                output_file,
                "{}fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{\n",
                indent
            )?;
            indent.inc();
            write!(output_file, "{}let arg0 = self;\n", indent)?;
            write!(output_file, "{}let value = ", indent)?;
            write_expr(*body, output_file, program, indent)?;
            write!(output_file, ";\n")?;
            write!(output_file, "{}write!(f, \"{{}}\", value.value)\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
        } else if class_name == "cmp" {
            let impl_ty = ir_type_to_rust_type(&ty, program);
            write!(
                output_file,
                "{}impl std::cmp::Ord for {} {{\n",
                indent, impl_ty,
            )?;
            indent.inc();
            write!(
                output_file,
                "{}fn cmp(&self, arg1: &{}) -> std::cmp::Ordering {{\n",
                indent, impl_ty
            )?;
            indent.inc();
            write!(output_file, "{}let arg0 = self;\n", indent)?;
            write!(output_file, "{}let value = ", indent)?;
            write_expr(*body, output_file, program, indent)?;
            write!(output_file, ";\n")?;
            write!(output_file, "{}match value {{\n", indent)?;
            indent.inc();
            write!(
                output_file,
                "{} {}::Less => std::cmp::Ordering::Less,\n",
                indent, result_ty
            )?;
            write!(
                output_file,
                "{} {}::Equal => std::cmp::Ordering::Equal,\n",
                indent, result_ty
            )?;
            write!(
                output_file,
                "{} {}::Greater => std::cmp::Ordering::Greater,\n",
                indent, result_ty
            )?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
        }
    } else {
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
                generate_builtin(
                    function,
                    output_file,
                    program,
                    indent,
                    original_name.as_ref(),
                    &result_type,
                    result_ty.as_ref(),
                    arg_types,
                )?;
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
            FunctionInfo::ExternClassImpl(..) => unreachable!(),
        }
        write!(output_file, "\n{}}}\n", indent,)?;
    }
    Ok(())
}

struct Module {
    name: String,
    functions: Vec<FunctionId>,
    typedefs: Vec<TypeDefId>,
    internal: bool,
}

impl Module {
    fn new(name: String) -> Module {
        let internal = name == MIR_INTERNAL_MODULE_NAME;
        Module {
            name: name,
            functions: Vec::new(),
            typedefs: Vec::new(),
            internal: internal,
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
        if self.internal {
            write!(output_file, "{}trait Function<A, B> {{\n", indent)?;
            indent.inc();
            write!(output_file, "{}fn call(&self, a: A) -> B;\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;
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
        let mut output_file = File::create(filename)?;
        write!(output_file, "#![allow(non_snake_case)]\n")?;
        write!(output_file, "#![allow(non_camel_case_types)]\n")?;
        write!(output_file, "#![allow(unused_variables)]\n")?;
        write!(output_file, "#![allow(dead_code)]\n\n")?;
        let mut rust_program = RustProgram::new();
        rust_program.get_module(MIR_INTERNAL_MODULE_NAME.to_string());
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
