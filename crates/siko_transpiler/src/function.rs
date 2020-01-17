use crate::builtins::generate_builtin;
use crate::closure::ClosureDataDef;
use crate::expr::write_expr;
use crate::types::ir_type_to_rust_type;
use crate::util::arg_name;
use crate::util::Indent;
use siko_mir::function::FunctionId;
use siko_mir::function::FunctionInfo;
use siko_mir::program::Program;
use std::io::Result;
use std::io::Write;

pub fn write_function(
    function_id: FunctionId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
    closure_data_defs: &mut Vec<ClosureDataDef>,
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
            write_expr(*body, output_file, program, indent, closure_data_defs)?;
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
            write_expr(*body, output_file, program, indent, closure_data_defs)?;
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
                write_expr(*body, output_file, program, indent, closure_data_defs)?;
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
