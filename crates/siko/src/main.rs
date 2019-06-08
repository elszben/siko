use siko_compiler;
use siko_constants;
use siko_interpreter;
use siko_ir;
use siko_location_info;
use siko_name_resolver;
use siko_parser;
use siko_syntax;
use siko_type_checker;
use siko_util;

use std::env;
use std::path::Path;

use siko_compiler::compiler::Compiler;
use siko_compiler::compiler::CompilerInput;
use siko_compiler::config::Config;
use walkdir::WalkDir;

fn process_args(args: Vec<String>) -> (Config, Vec<CompilerInput>) {
    let mut inputs = Vec::new();
    let mut config = Config::new();
    for arg in args {
        if arg == "-v" {
            config.verbose = true;
        } else {
            let path = Path::new(&arg);
            if path.is_dir() {
                for entry in WalkDir::new(path) {
                    let entry = entry.unwrap();
                    if let Some(ext) = entry.path().extension() {
                        if ext == "sk" {
                            let input = CompilerInput::File {
                                name: format!("{}", entry.path().display()),
                            };
                            inputs.push(input);
                        }
                    }
                }
            } else if path.is_file() {
                let input = CompilerInput::File { name: arg };
                inputs.push(input);
            }
        }
    }
    //println!("Compiling {} file(s)", inputs.len());
    (config, inputs)
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    let (config, inputs) = process_args(args);

    let mut compiler = Compiler::new(config);

    if let Err(e) = compiler.compile(inputs) {
        compiler.report_error(e);
    }
}
