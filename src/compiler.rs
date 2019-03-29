use crate::error::Error;
use crate::file_manager::FileManager;
//use crate::interpreter::Interpreter;
use crate::location_info::filepath::FilePath;
use crate::location_info::location_info::LocationInfo;
use crate::name_resolution::resolver::Resolver;
use crate::parser::lexer::Lexer;
use crate::parser::parser::Parser;
use crate::syntax::program::Program;
use crate::typechecker::typechecker::Typechecker;

pub enum CompilerInput {
    File {
        name: String,
    },
    #[allow(unused)]
    Memory {
        name: String,
        content: String,
    },
}

fn parse(
    content: &str,
    verbose: bool,
    file_path: FilePath,
    program: &mut Program,
    location_info: &mut LocationInfo,
) -> Result<(), Error> {
    //println!("Compiling {}", file_path.path);
    let mut lexer = Lexer::new(content, file_path.clone());
    let tokens = lexer.process()?;
    let token_kinds: Vec<_> = tokens
        .iter()
        .map(|t| format!("{:?}", t.token.kind()))
        .collect();
    if verbose {
        println!("Tokens [{}]", token_kinds.join(", "));
    }
    let mut parser = Parser::new(file_path, &tokens[..], program, location_info);
    parser.parse()?;
    if verbose {
        println!("program {:?}", program);
    }
    Ok(())
}

pub struct Compiler {
    file_manager: FileManager,
    location_info: LocationInfo,
    verbose: bool,
}

impl Compiler {
    pub fn new(verbose: bool) -> Compiler {
        Compiler {
            file_manager: FileManager::new(),
            location_info: LocationInfo::new(),
            verbose: verbose,
        }
    }

    pub fn compile(&mut self, inputs: Vec<CompilerInput>) -> Result<(), Error> {
        let mut program = Program::new();

        for input in inputs.iter() {
            match input {
                CompilerInput::File { name } => {
                    self.file_manager.read(FilePath::new(name.to_string()))?;
                }
                CompilerInput::Memory { name, content } => {
                    self.file_manager
                        .add_from_memory(FilePath::new(name.to_string()), content.clone());
                }
            }
        }

        for (file_path, content) in self.file_manager.files.iter() {
            parse(
                content,
                self.verbose,
                file_path.clone(),
                &mut program,
                &mut self.location_info,
            )?;
        }

        let mut resolver = Resolver::new();

        let ir_program = resolver.resolve(&program)?;

        if self.verbose {
            println!("program {:#?}", ir_program);
        }

        let mut typechecker = Typechecker::new();

        typechecker.check(&ir_program)?;
        /*
            let interpreter = Interpreter::new();

            let value = interpreter.run(&ir_program)?;

            println!("Result {}", value);
        */
        Ok(())
    }

    pub fn report_error(&self, error: Error) {
        error.report_error(&self.file_manager, &self.location_info);
    }
}
