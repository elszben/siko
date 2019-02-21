mod constants;
mod error;
mod file_manager;
//mod interpreter;
mod ir;
mod lexer;
mod location_info;
mod name_resolution;
mod parser;
mod syntax;
mod token;
//mod typechecker;
mod util;

use crate::error::Error;
use crate::file_manager::FileManager;
//use crate::interpreter::Interpreter;
use crate::lexer::Lexer;
use crate::location_info::filepath::FilePath;
use crate::location_info::location_info::LocationInfo;
use crate::name_resolution::resolver::Resolver;
use crate::parser::parser::Parser;
use crate::syntax::program::Program;
//use crate::typechecker::typechecker::Typechecker;
use std::env;

fn parse(
    content: &str,
    verbose: bool,
    file_path: FilePath,
    program: &mut Program,
    location_info: &mut LocationInfo,
) -> Result<(), Error> {
    println!("Compiling {}", file_path.path);
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

fn compile(
    files: Vec<&String>,
    verbose: bool,
    file_manager: &mut FileManager,
    location_info: &mut LocationInfo,
) -> Result<(), Error> {
    let mut program = Program::new();

    for arg in files.iter() {
        file_manager.read(FilePath::new(arg.to_string()))?;
    }

    for (file_path, content) in file_manager.files.iter() {
        parse(
            content,
            verbose,
            file_path.clone(),
            &mut program,
            location_info,
        )?;
    }

    let mut resolver = Resolver::new();

    let ir_program = resolver.resolve(&program)?;

    if verbose {
        println!("program {:#?}", ir_program);
    }
    /*
        let typechecker = Typechecker::new();

        typechecker.check(&ir_program)?;

        let interpreter = Interpreter::new();

        let value = interpreter.run(&ir_program)?;

        println!("Result {}", value);
    */
    Ok(())
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let args2: Vec<_> = args.iter().filter(|a| *a != "-v").collect();
    let verbose = args2.len() != args.len();
    let mut file_manager = FileManager::new();
    let mut location_info = LocationInfo::new();
    if let Err(e) = compile(args2, verbose, &mut file_manager, &mut location_info) {
        e.report_error(&file_manager, &location_info);
    }
}
