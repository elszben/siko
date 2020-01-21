use crate::closure::ClosureDataDef;
use crate::closure::DynTrait;
use crate::pattern::write_pattern;
use crate::types::ir_type_to_rust_type;
use crate::util::arg_name;
use crate::util::get_module_name;
use crate::util::Indent;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::expr::Expr;
use siko_mir::expr::ExprId;
use siko_mir::pattern::Pattern;
use siko_mir::program::Program;
use std::io::Result;
use std::io::Write;

pub fn write_expr(
    expr_id: ExprId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    closure_data_defs: &mut Vec<ClosureDataDef>,
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
                let is_statement =
                    write_expr(*item, output_file, program, indent, closure_data_defs)?;
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
                write_expr(*item, output_file, program, indent, closure_data_defs)?;
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
            write_expr(*receiver, output_file, program, indent, closure_data_defs)?;
            write!(output_file, ";\n")?;
            for (item, index) in items {
                let field = &record.fields[*index];
                write!(output_file, "{}value.{} = ", indent, field.name)?;
                write_expr(*item, output_file, program, indent, closure_data_defs)?;
                write!(output_file, ";\n")?;
            }
            write!(output_file, "{}value }}", indent)?;
            indent.dec();
        }
        Expr::Bind(pattern, rhs) => {
            write!(output_file, "let ")?;
            write_pattern(*pattern, output_file, program, indent, closure_data_defs)?;
            write!(output_file, " = ")?;
            write_expr(*rhs, output_file, program, indent, closure_data_defs)?;
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
            if function.arg_count > args.len() {
                let mut arg_types = Vec::new();
                function.function_type.get_args(&mut arg_types);
                let mut result_type = function.function_type.clone();
                let mut closure_type = result_type.clone();
                let mut fields = Vec::new();
                let mut traits = Vec::new();
                for index in 0..function.arg_count {
                    let mut arg_types = Vec::new();
                    result_type.get_args(&mut arg_types);
                    result_type = result_type.get_result_type(1);
                    if index == args.len() - 1 {
                        closure_type = result_type.clone();
                    }
                    let arg_type = &arg_types[0];
                    if index >= args.len() {
                        let arg_type_str = ir_type_to_rust_type(arg_type, program);
                        let result_type_str = ir_type_to_rust_type(&result_type, program);
                        if index == function.arg_count - 1 {
                            let mut real_call_args = Vec::new();
                            for i in 0..function.arg_count - 1 {
                                if i < args.len() {
                                    let arg = format!("self.get_arg{}()", i);
                                    real_call_args.push(arg);
                                } else {
                                    let arg = format!(
                                        "self.{}.as_ref().expect(\"Missing arg\").clone()",
                                        arg_name(i)
                                    );
                                    real_call_args.push(arg);
                                }
                            }
                            real_call_args.push(format!("arg0"));
                            let fn_name = format!(
                                "crate::{}::{}",
                                get_module_name(&function.module),
                                function.name
                            );
                            let call_str = format!("{}({})", fn_name, real_call_args.join(", "));
                            traits.push(DynTrait::RealCall(
                                arg_type_str,
                                result_type_str,
                                call_str,
                            ));
                        } else {
                            traits.push(DynTrait::ArgSave(
                                arg_type_str,
                                result_type_str,
                                arg_name(index),
                            ));
                        }
                    }
                    if index < function.arg_count - 1 {
                        let field_name = arg_name(index);
                        let field_type = if index >= args.len() {
                            format!("Option<{}>", ir_type_to_rust_type(arg_type, program))
                        } else {
                            ir_type_to_rust_type(arg_type, program)
                        };
                        fields.push((field_name, field_type));
                    }
                }
                let name = if let Some(closure) = program.closures.get(&closure_type) {
                    let name = format!("ClosureData{}", expr_id.id);
                    write!(
                        output_file,
                        "crate::{}::{} {{ value: Box::new(crate::{}::{}{{",
                        get_module_name(MIR_INTERNAL_MODULE_NAME),
                        closure.name,
                        get_module_name(MIR_INTERNAL_MODULE_NAME),
                        name
                    )?;
                    name
                } else {
                    let ss = ir_type_to_rust_type(&result_type, program);
                    panic!("No closure for type {}", ss);
                };
                for (index, field) in fields.iter().enumerate() {
                    write!(output_file, "{} : ", field.0)?;
                    if index < args.len() {
                        let arg = &args[index];
                        write_expr(*arg, output_file, program, indent, closure_data_defs)?;
                    } else {
                        write!(output_file, "None")?;
                    }
                    if index != fields.len() - 1 {
                        write!(output_file, ", ")?;
                    }
                }
                write!(output_file, "}})}}")?;
                let closure_data_def = ClosureDataDef {
                    name: name,
                    fields: fields,
                    traits: traits,
                };
                closure_data_defs.push(closure_data_def);
            } else if function.arg_count < args.len() {
                let name = format!(
                    "crate::{}::{}",
                    get_module_name(&function.module),
                    function.name
                );
                write!(output_file, "{{{}let dyn_fn = ", indent)?;
                write!(output_file, "{} (", name)?;
                for index in 0..function.arg_count {
                    let arg = &args[index];
                    write_expr(*arg, output_file, program, indent, closure_data_defs)?;
                    write!(output_file, ".clone()")?;
                    if index != args.len() - 1 {
                        write!(output_file, ", ")?;
                    }
                }
                write!(output_file, ");\n")?;
                for index in function.arg_count..args.len() {
                    write!(output_file, "{}let dyn_fn = dyn_fn.call(", indent)?;
                    let arg = &args[index];
                    write_expr(*arg, output_file, program, indent, closure_data_defs)?;
                    write!(output_file, ");\n",)?;
                }
                write!(output_file, "{}dyn_fn\n", indent)?;
                write!(output_file, "{}}}", indent)?;
            } else {
                assert_eq!(function.arg_count, args.len());
                let name = format!(
                    "crate::{}::{}",
                    get_module_name(&function.module),
                    function.name
                );
                write!(output_file, "{} (", name)?;
                for (index, arg) in args.iter().enumerate() {
                    write_expr(*arg, output_file, program, indent, closure_data_defs)?;
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
                write_expr(*arg, output_file, program, indent, closure_data_defs)?;
                write!(output_file, ".value")?;
                if index != args.len() - 1 {
                    write!(output_file, ",")?;
                }
            }
            write!(output_file, ")}}")?;
        }
        Expr::CaseOf(body, cases) => {
            write!(output_file, "match (")?;
            write_expr(*body, output_file, program, indent, closure_data_defs)?;
            write!(output_file, ") {{\n")?;
            indent.inc();
            for case in cases {
                write!(output_file, "{}", indent)?;
                write_pattern(
                    case.pattern_id,
                    output_file,
                    program,
                    indent,
                    closure_data_defs,
                )?;
                write!(output_file, " => {{")?;
                write_expr(case.body, output_file, program, indent, closure_data_defs)?;
                write!(output_file, "}}\n")?;
            }
            indent.dec();
            write!(output_file, "{}}}", indent)?;
        }
        Expr::If(cond, true_branch, false_branch) => {
            let ty = program.get_expr_type(cond);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "if {{ match (")?;
            write_expr(*cond, output_file, program, indent, closure_data_defs)?;
            write!(
                output_file,
                ") {{ {}::True => true, {}::False => false, }} }} ",
                ty, ty
            )?;
            write!(output_file, " {{ ")?;
            write_expr(
                *true_branch,
                output_file,
                program,
                indent,
                closure_data_defs,
            )?;
            write!(output_file, " }} ")?;
            write!(output_file, " else {{ ")?;
            write_expr(
                *false_branch,
                output_file,
                program,
                indent,
                closure_data_defs,
            )?;
            write!(output_file, " }} ")?;
        }
        Expr::FieldAccess(index, receiver) => {
            let ty = program.get_expr_type(receiver);
            let id = ty.get_typedef_id();
            let record = program.typedefs.get(&id).get_record();
            let field = &record.fields[*index];
            write_expr(*receiver, output_file, program, indent, closure_data_defs)?;
            write!(output_file, ".{}", field.name)?;
        }
        Expr::List(items) => {
            let ty = program.get_expr_type(&expr_id);
            let ty = ir_type_to_rust_type(ty, program);
            write!(output_file, "{} {{ value: vec![", ty)?;
            for (index, item) in items.iter().enumerate() {
                write_expr(*item, output_file, program, indent, closure_data_defs)?;
                if index != items.len() - 1 {
                    write!(output_file, ", ")?;
                }
            }
            write!(output_file, "] }}")?;
        }
        Expr::DynamicFunctionCall(receiver, args) => {
            indent.inc();
            write!(output_file, "{{\n{}let dyn_fn = ", indent)?;
            write_expr(*receiver, output_file, program, indent, closure_data_defs)?;
            write!(output_file, ";\n")?;
            for arg in args {
                write!(output_file, "{}let dyn_fn = dyn_fn.call(", indent)?;
                write_expr(*arg, output_file, program, indent, closure_data_defs)?;
                write!(output_file, ");\n")?;
            }
            write!(output_file, "{}dyn_fn\n", indent)?;
            indent.dec();
            write!(output_file, "{}}}", indent)?;
        }
        Expr::Clone(rhs) => {
            write_expr(*rhs, output_file, program, indent, closure_data_defs)?;
            write!(output_file, ".clone()")?;
        }
    }
    Ok(is_statement)
}
