use siko_mir::function::FunctionId;
use siko_mir::program::Program;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Result;
use std::io::Write;

fn get_module_name(name: &str) -> String {
    name.replace(".", "_")
}

fn write_function(
    function_id: FunctionId,
    output_file: &mut dyn Write,
    program: &Program,
) -> Result<()> {
    let function = program.functions.get(&function_id);
    write!(output_file, "pub fn {}() {{\n", function.name)?;
    write!(output_file, "}}\n",)?;
    Ok(())
}

struct Module {
    name: String,
    functions: Vec<FunctionId>,
}

impl Module {
    fn new(name: String) -> Module {
        Module {
            name: name,
            functions: Vec::new(),
        }
    }

    fn write(&self, output_file: &mut dyn Write, program: &Program) -> Result<()> {
        write!(output_file, "mod {} {{\n", get_module_name(&self.name))?;
        for function_id in &self.functions {
            write_function(*function_id, output_file, program)?;
        }
        write!(output_file, "}}\n",)?;
        Ok(())
    }
}

pub struct Transpiler {}

impl Transpiler {
    pub fn process(program: &Program, target_file: &str) -> Result<()> {
        let filename = format!("{}.rs", target_file);
        println!("Transpiling to {}", filename);
        let mut output_file = File::create(filename)?;
        let mut modules = BTreeMap::new();
        for (id, function) in program.functions.items.iter() {
            let module = modules
                .entry(function.module.clone())
                .or_insert_with(|| Module::new(function.module.clone()));
            module.functions.push(*id);
        }
        for (_, module) in modules {
            module.write(&mut output_file, program)?;
        }
        write!(output_file, "fn main() {{\n")?;
        write!(output_file, "Main::main();\n")?;
        write!(output_file, "}}\n")?;
        Ok(())
    }
}
