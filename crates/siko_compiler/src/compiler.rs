use crate::config::Config;
use crate::error::Error;
use siko_interpreter::interpreter::Interpreter;
use siko_ir::expr_visualizer::ExprVisualizer;
use siko_location_info::error_context::ErrorContext;
use siko_location_info::file_manager::FileManager;
use siko_location_info::filepath::FilePath;
use siko_location_info::location_info::LocationInfo;
use siko_name_resolver::resolver::Resolver;
use siko_parser::lexer::Lexer;
use siko_parser::parser::Parser;
use siko_syntax::program::Program;
use siko_type_checker::typechecker::Typechecker;
use siko_type_checker2::typechecker::Typechecker as Typechecker2;
use siko_util::ElapsedTimeMeasure;
use siko_util::ElapsedTimeMeasureCollector;

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
    let mut parser = Parser::new(file_path, &tokens[..], program, location_info);
    parser.parse()?;
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

        {
            let _m = ElapsedTimeMeasure::new("FileReader");
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
        }

        {
            let _m = ElapsedTimeMeasure::new("Parser");
            for (file_path, content) in self.file_manager.files.iter() {
                parse(
                    content,
                    file_path.clone(),
                    &mut program,
                    &mut self.location_info,
                )?;
            }
        }

        let (mut ir_program, counter) = {
            let _m = ElapsedTimeMeasure::new("NameResolver");
            let mut resolver = Resolver::new();

            resolver.resolve(&program)?
        };

        {
            let _m = ElapsedTimeMeasure::new("Typechecker");
            let typechecker = Typechecker2::new();

            typechecker.check(&mut ir_program, counter)?;
        }

        {
            let _m = ElapsedTimeMeasure::new("Typechecker");
            let typechecker = Typechecker::new();

            typechecker.check(&mut ir_program)?;
        }

        if self.config.visualize {
            for (id, _) in &ir_program.functions.items {
                ExprVisualizer::generate(&id, &ir_program);
            }
        }

        ElapsedTimeMeasureCollector::print_instance_resolver_time();

        Interpreter::run(ir_program, self.context());

        //println!("Result {}", value);
        Ok(())
    }

    fn context(&self) -> ErrorContext {
        ErrorContext {
            file_manager: self.file_manager.clone(),
            location_info: self.location_info.clone(),
        }
    }

    pub fn report_error(&self, error: Error) {
        error.report_error(&self.context());
    }
}
