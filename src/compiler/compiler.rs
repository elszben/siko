use crate::compiler::config::Config;
use crate::compiler::file_manager::FileManager;
use crate::error::Error;
use crate::error::ErrorContext;
use crate::interpreter::interpreter::Interpreter;
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
    config: &Config,
    file_path: FilePath,
    program: &mut Program,
    location_info: &mut LocationInfo,
) -> Result<(), Error> {
    //println!("Compiling {}", file_path.path);
    let mut lexer = Lexer::new(content, file_path.clone());
    let mut errors = Vec::new();
    let tokens = match lexer.process(&mut errors) {
        Ok(tokens) => {
            if errors.is_empty() {
                tokens
            } else {
                return Err(Error::LexerError(errors));
            }
        }
        Err(e) => {
            errors.push(e);
            return Err(Error::LexerError(errors));
        }
    };
    let token_kinds: Vec<_> = tokens
        .iter()
        .map(|t| format!("{:?}", t.token.kind()))
        .collect();
    if config.verbose {
        println!("Tokens [{}]", token_kinds.join(", "));
    }
    let mut parser = Parser::new(file_path, &tokens[..], program, location_info);
    parser.parse()?;
    if config.verbose {
        println!("program {:?}", program);
    }
    Ok(())
}

pub struct Compiler {
    file_manager: FileManager,
    location_info: LocationInfo,
    config: Config,
}

impl Compiler {
    pub fn new(config: Config) -> Compiler {
        Compiler {
            file_manager: FileManager::new(),
            location_info: LocationInfo::new(),
            config: config,
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
                &self.config,
                file_path.clone(),
                &mut program,
                &mut self.location_info,
            )?;
        }

        let mut resolver = Resolver::new();

        let ir_program = resolver.resolve(&program)?;

        if self.config.verbose {
            println!("program {:#?}", ir_program);
        }

        let mut typechecker = Typechecker::new();

        // typechecker.check(&ir_program)?;
        let mut interpreter = Interpreter::new(self.context());

        interpreter.run(&ir_program);

        //println!("Result {}", value);
        Ok(())
    }

    fn context(&self) -> ErrorContext {
        ErrorContext {
            file_manager: &self.file_manager,
            location_info: &self.location_info,
        }
    }

    pub fn report_error(&self, error: Error) {
        error.report_error(&self.context());
    }
}
