use siko_constants::MIR_FUNCTION_TRAIT_NAME;
use siko_constants::MIR_INTERNAL_MODULE_NAME;
use siko_mir::data::TypeDef;
use siko_mir::data::TypeDefId;
use siko_mir::function::FunctionId;
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
            Type::Named(name, id) => {
                let typedef = program.typedefs.get(id);
                let module_name = match typedef {
                    TypeDef::Adt(adt) => get_module_name(&adt.module),
                    TypeDef::Record(record) => get_module_name(&record.module),
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
    Ok(())
}

fn write_function(
    function_id: FunctionId,
    output_file: &mut dyn Write,
    program: &Program,
    indent: &mut Indent,
) -> Result<()> {
    let function = program.functions.get(&function_id);
    write!(output_file, "{}pub fn {}() {{\n", indent, function.name)?;
    write!(output_file, "{}}}\n", indent,)?;
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
        write!(output_file, "{}Main::main();\n", indent)?;
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

trait Function<A, B> {
    fn call(&self);
}

struct Foo {
    alma: Box<dyn Function<i64, dyn Function<i64, i64>>>,
}
