use crate::function::write_function;
use crate::internal_module::write_internal_defs;
use crate::typedef::write_typedef;
use crate::util::get_module_name;
use crate::util::Indent;
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
    ) -> Result<()> {
        write!(output_file, "mod {} {{\n", get_module_name(&self.name))?;
        indent.inc();
        for typedef_id in &self.typedefs {
            write_typedef(*typedef_id, output_file, program, indent)?;
        }
        if self.internal {
            write_internal_defs(output_file, program, indent)?;
        }
        for function_id in &self.functions {
            write_function(*function_id, output_file, program, indent)?;
        }
        indent.dec();
        write!(output_file, "}}\n\n",)?;
        Ok(())
    }
}
