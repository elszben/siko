use siko_compiler::compiler::Compiler;
use siko_compiler::compiler::CompilerInput;
use siko_compiler::config::Config;
use std::env;
use std::path::Path;
use walkdir::WalkDir;

fn process_args(args: Vec<String>) -> (Config, Vec<CompilerInput>) {
    let mut inputs = Vec::new();
    let mut config = Config::new();
    let prelude_source = include_str!("std/prelude.sk");
    let prelude = CompilerInput::Memory{
        name: "prelude".to_string(),
        content: prelude_source.to_string()
    };
    inputs.push(prelude);
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
