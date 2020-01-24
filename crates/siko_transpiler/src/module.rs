use crate::function::write_function;
use crate::typedef::write_typedef;
use crate::types::ir_type_to_rust_type;
use crate::util::arg_name;
use crate::util::get_module_name;
use crate::util::Indent;
use siko_constants::MIR_FUNCTION_TRAIT_NAME;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::data::TypeDefId;
use siko_mir::function::FunctionId;
use siko_mir::program::Program;
use siko_mir::types::DynamicCallTrait;
use std::io::Result;
use std::io::Write;

pub struct Module {
    name: String,
    pub functions: Vec<FunctionId>,
    pub typedefs: Vec<TypeDefId>,
    pub internal: bool,
}

impl Module {
    pub fn new(name: String) -> Module {
        let internal = name == MIR_INTERNAL_MODULE_NAME;
        Module {
            name: name,
            functions: Vec::new(),
            typedefs: Vec::new(),
            internal: internal,
        }
    }

    pub fn write(
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
        if self.internal {
            for (_, partial_function_call) in program.partial_function_calls.items.iter() {
                write!(
                    output_file,
                    "{}impl Clone for {} {{\n",
                    indent,
                    partial_function_call.get_name()
                )?;
                indent.inc();
                write!(
                    output_file,
                    "{}fn clone(&self) -> {} {{\n",
                    indent,
                    partial_function_call.get_name()
                )?;
                indent.inc();
                write!(
                    output_file,
                    "{}{} {{\n",
                    indent,
                    partial_function_call.get_name()
                )?;
                indent.inc();
                for index in 0..partial_function_call.fields.len() {
                    write!(
                        output_file,
                        "{}{}: self.{}.clone(),\n",
                        indent,
                        arg_name(index),
                        arg_name(index),
                    )?;
                }
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                write!(
                    output_file,
                    "{}pub struct {} {{\n",
                    indent,
                    partial_function_call.get_name()
                )?;
                indent.inc();
                for (index, field) in partial_function_call.fields.iter().enumerate() {
                    if field.deferred {
                        write!(
                            output_file,
                            "{} pub {}: Option<{}>,\n",
                            indent,
                            arg_name(index),
                            ir_type_to_rust_type(&field.ty, program)
                        )?;
                    } else {
                        write!(
                            output_file,
                            "{} pub {}: {},\n",
                            indent,
                            arg_name(index),
                            ir_type_to_rust_type(&field.ty, program)
                        )?;
                    }
                }
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;

                for dyn_trait in &partial_function_call.traits {
                    match dyn_trait {
                        DynamicCallTrait::ArgSave {
                            from,
                            to,
                            field_index,
                        } => {
                            write!(
                                output_file,
                                "{}impl {}<{}, {}> for {} {{\n",
                                indent,
                                MIR_FUNCTION_TRAIT_NAME,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                                partial_function_call.get_name()
                            )?;
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn call(&self, arg0: {}) -> {} {{\n",
                                indent,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                            )?;
                            indent.inc();
                            write!(output_file, "{}let mut clone = self.clone();\n", indent)?;
                            write!(
                                output_file,
                                "{}clone.{} = Some(arg0);\n",
                                indent,
                                arg_name(*field_index)
                            )?;
                            write!(
                                output_file,
                                "{}{} {{ value: Box::new(clone) }}\n",
                                indent,
                                ir_type_to_rust_type(to, program),
                            )?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn box_clone(&self) -> Box<dyn {}<{},{}>> {{\n",
                                indent,
                                MIR_FUNCTION_TRAIT_NAME,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                            )?;
                            indent.inc();
                            write!(output_file, "{}Box::new(self.clone())\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                        }
                        DynamicCallTrait::RealCall { from, to } => {
                            write!(
                                output_file,
                                "{}impl {}<{}, {}> for {} {{\n",
                                indent,
                                MIR_FUNCTION_TRAIT_NAME,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                                partial_function_call.get_name()
                            )?;
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn call(&self, arg0: {}) -> {} {{\n",
                                indent,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                            )?;
                            indent.inc();
                            let function = program.functions.get(&partial_function_call.function);
                            write!(
                                output_file,
                                "{}crate::{}::{}(",
                                indent, function.module, function.name
                            )?;
                            for (index, field) in partial_function_call.fields.iter().enumerate() {
                                if field.deferred {
                                    write!(
                                        output_file,
                                        "self.{}.as_ref().expect(\"Missing arg\").clone(), ",
                                        arg_name(index)
                                    )?;
                                } else {
                                    write!(output_file, "self.{}.clone(), ", arg_name(index))?;
                                }
                            }
                            write!(output_file, "arg0)\n")?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn box_clone(&self) -> Box<dyn {}<{},{}>> {{\n",
                                indent,
                                MIR_FUNCTION_TRAIT_NAME,
                                ir_type_to_rust_type(from, program),
                                ir_type_to_rust_type(to, program),
                            )?;
                            indent.inc();
                            write!(output_file, "{}Box::new(self.clone())\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                        }
                    }
                }
            }
        }
        for function_id in &self.functions {
            write_function(*function_id, output_file, program, indent)?;
        }
        if self.internal {
            write!(
                output_file,
                "{}pub trait {}<A, B> {{\n",
                indent, MIR_FUNCTION_TRAIT_NAME
            )?;
            indent.inc();
            write!(output_file, "{}fn call(&self, a: A) -> B;\n", indent)?;
            write!(
                output_file,
                "{}fn box_clone(&self) -> Box<dyn Function<A,B>>;\n",
                indent
            )?;
            indent.dec();
            write!(output_file, "{}}}\n", indent)?;

            for (_, closure) in &program.closures {
                write!(output_file, "{}pub struct {} {{\n", indent, closure.name)?;
                let fn_name = ir_type_to_rust_type(&closure.ty, program);
                indent.inc();
                write!(output_file, "{}pub value: {}\n", indent, fn_name,)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;

                write!(
                    output_file,
                    "{}impl Clone for {} {{\n",
                    indent, closure.name
                )?;
                indent.inc();
                write!(
                    output_file,
                    "{}fn clone(&self) -> {} {{\n",
                    indent, closure.name
                )?;
                indent.inc();
                write!(output_file, "{}{} {{\n", indent, closure.name)?;
                indent.inc();
                write!(output_file, "{}value: self.value.box_clone(),\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;

                write!(output_file, "{}impl {} {{\n", indent, closure.name)?;
                indent.inc();
                write!(
                    output_file,
                    "{}pub fn call(&self, arg0: {}) -> {} {{\n",
                    indent,
                    ir_type_to_rust_type(&closure.from_ty, program),
                    ir_type_to_rust_type(&closure.to_ty, program)
                )?;
                indent.inc();
                write!(output_file, "{}self.value.call(arg0)\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;
            }
        }
        indent.dec();
        write!(output_file, "}}\n\n",)?;
        Ok(())
    }
}
