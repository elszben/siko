mod constants;
mod error;
mod file_manager;
//mod interpreter;
mod compiler;
mod ir;
mod location_info;
mod name_resolution;
mod parser;
mod syntax;
mod test;
mod token;
mod typechecker;
mod util;

use std::env;

use crate::compiler::Compiler;
use crate::compiler::CompilerInput;

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    let args2: Vec<_> = args.iter().filter(|a| *a != "-v").collect();
    let verbose = args2.len() != args.len();

    let mut compiler = Compiler::new(verbose);

    let inputs = args2
        .iter()
        .map(|n| CompilerInput::File {
            name: n.to_string(),
        })
        .collect();

    if let Err(e) = compiler.compile(inputs) {
        compiler.report_error(e);
    }
}
