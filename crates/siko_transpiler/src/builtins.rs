use crate::types::ir_type_to_rust_type;
use crate::util::Indent;
use siko_mir::function::Function;
use siko_mir::program::Program;
use siko_mir::types::Type;
use std::io::Result;
use std::io::Write;

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
    for v in &adt_opt.variants {
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
    _program: &Program,
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
    _function: &Function,
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
    _function: &Function,
    output_file: &mut dyn Write,
    _program: &Program,
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

pub fn generate_builtin(
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
