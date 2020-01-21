use crate::closure::ClosureDataDef;
use crate::closure::DynTrait;
use crate::function::write_function;
use crate::typedef::write_typedef;
use crate::types::ir_fn_type_to_rust_fn_type;
use crate::types::ir_type_to_rust_type;
use crate::util::get_module_name;
use crate::util::Indent;
use siko_constants::MIR_FUNCTION_TRAIT_NAME;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::data::TypeDefId;
use siko_mir::function::FunctionId;
use siko_mir::program::Program;
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
        closure_data_defs: &mut Vec<ClosureDataDef>,
    ) -> Result<()> {
        write!(output_file, "mod {} {{\n", get_module_name(&self.name))?;
        indent.inc();
        for typedef_id in &self.typedefs {
            write_typedef(*typedef_id, output_file, program, indent)?;
        }
        if self.internal {
            for closure_data_def in closure_data_defs.iter() {
                write!(
                    output_file,
                    "{}impl Clone for {} {{\n",
                    indent, closure_data_def.name
                )?;
                indent.inc();
                write!(
                    output_file,
                    "{}fn clone(&self) -> {} {{\n",
                    indent, closure_data_def.name
                )?;
                indent.inc();
                write!(output_file, "{}{} {{\n", indent, closure_data_def.name)?;
                indent.inc();
                for field in &closure_data_def.fields {
                    write!(
                        output_file,
                        "{}{}: self.{}.clone(),\n",
                        indent, field.0, field.0
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
                    indent, closure_data_def.name
                )?;
                indent.inc();
                for (name, ty_str) in &closure_data_def.fields {
                    write!(output_file, "{} pub {}: {},\n", indent, name, ty_str)?;
                }
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;

                write!(output_file, "{}impl {} {{\n", indent, closure_data_def.name)?;
                indent.inc();
                for (name, ty_str) in &closure_data_def.fields {
                    write!(
                        output_file,
                        "{} pub fn get_{}(&self) -> {} {{\n",
                        indent, name, ty_str
                    )?;
                    indent.inc();
                    write!(output_file, "{}self.{}.clone()\n", indent, name)?;
                    indent.dec();
                    write!(output_file, "{}}}\n", indent)?;
                }
                indent.dec();
                write!(output_file, "{}}}\n", indent)?;

                for dyn_trait in &closure_data_def.traits {
                    match dyn_trait {
                        DynTrait::ArgSave(from, to, arg) => {
                            write!(
                                output_file,
                                "{}impl {}<{}, {}> for {} {{\n",
                                indent, MIR_FUNCTION_TRAIT_NAME, from, to, closure_data_def.name
                            )?;
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn call(&self, arg0: {}) -> {} {{\n",
                                indent, from, to
                            )?;
                            indent.inc();
                            write!(output_file, "{}let mut clone = self.clone();\n", indent)?;
                            write!(output_file, "{}clone.{} = Some(arg0);\n", indent, arg)?;
                            write!(
                                output_file,
                                "{}{} {{ value: Box::new(clone) }}\n",
                                indent, to
                            )?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn box_clone(&self) -> Box<dyn {}<{},{}>> {{\n",
                                indent, MIR_FUNCTION_TRAIT_NAME, from, to
                            )?;
                            indent.inc();
                            write!(output_file, "{}Box::new(self.clone())\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                        }
                        DynTrait::RealCall(from, to, call_str) => {
                            write!(
                                output_file,
                                "{}impl {}<{}, {}> for {} {{\n",
                                indent, MIR_FUNCTION_TRAIT_NAME, from, to, closure_data_def.name
                            )?;
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn call(&self, arg0: {}) -> {} {{\n",
                                indent, from, to
                            )?;
                            indent.inc();
                            write!(output_file, "{}{}\n", indent, call_str)?;
                            indent.dec();
                            write!(output_file, "{}}}\n", indent)?;
                            indent.dec();
                            indent.inc();
                            write!(
                                output_file,
                                "{}fn box_clone(&self) -> Box<dyn {}<{},{}>> {{\n",
                                indent, MIR_FUNCTION_TRAIT_NAME, from, to
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
            write_function(
                *function_id,
                output_file,
                program,
                indent,
                closure_data_defs,
            )?;
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
                let fn_name = ir_fn_type_to_rust_fn_type(&closure.ty, program);
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
