use colored::*;
use siko_compiler::compiler::Compiler;
use siko_compiler::compiler::CompilerInput;
use siko_compiler::config::Config;
use siko_constants::PRELUDE_NAME;
use std::env;
use std::path::Path;
use walkdir::WalkDir;

fn process_args(args: Vec<String>) -> (Config, Vec<CompilerInput>, bool) {
    let mut inputs = Vec::new();
    let mut config = Config::new();
    let mut success = true;
    let prelude_source = include_str!("std/prelude.sk");
    let prelude = CompilerInput::Memory {
        name: PRELUDE_NAME.to_string(),
        content: prelude_source.to_string(),
    };
    let std_source = include_str!("std/std.sk");
    let std = CompilerInput::Memory {
        name: "std.sk".to_string(),
        content: std_source.to_string(),
    };
    inputs.push(prelude);
    inputs.push(std);
    for arg in args {
        match arg.as_ref() {
            "-v" => {
                config.verbose = true;
            }
            "-i" => {
                config.visualize = true;
            }
            _ => {
                let path = Path::new(&arg);
                if !path.exists() {
                    let path_str = format!("{}", path.display());
                    eprintln!(
                        "{} path {} does not exist",
                        "ERROR:".red(),
                        path_str.yellow()
                    );
                    success = false;
                    continue;
                }
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
    }
    //println!("Compiling {} file(s)", inputs.len());
    (config, inputs, success)
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    let (config, inputs, success) = process_args(args);

    if !success {
        return;
    }

    let mut compiler = Compiler::new(config);

    if let Err(e) = compiler.compile(inputs) {
        compiler.report_error(e);
    }
}
