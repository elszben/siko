use colored::*;
use siko_compiler::compiler::Compiler;
use siko_compiler::compiler::CompilerInput;
use siko_compiler::config::Config;
use std::env;
use std::path::Path;
use walkdir::WalkDir;

fn process_dir(arg: String, inputs: &mut Vec<CompilerInput>) -> bool {
    let path = Path::new(&arg);
    if !path.exists() {
        let path_str = format!("{}", path.display());
        eprintln!(
            "{} path {} does not exist",
            "ERROR:".red(),
            path_str.yellow()
        );
        return false;
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
    true
}

fn process_args(args: Vec<String>) -> (Config, Vec<CompilerInput>, bool) {
    let mut inputs = Vec::new();
    let mut config = Config::new();
    let mut success = true;
    let mut std_path = format!("std");
    let mut file_given = false;
    for (index, arg) in args.iter().enumerate() {
        match arg.as_ref() {
            "-m" => {
                config.measure_durations = true;
            }
            "-i" => {
                config.visualize = true;
            }
            "-s" => {
                if index + 1 >= args.len() {
                    eprintln!("{} missing path after -s", "ERROR:".red(),);
                    success = false;
                } else {
                    std_path = args[index + 1].to_string();
                }
            }
            "-h" => {
                println!("arguments: <filename>+|<options>");
                println!("-m measure durations");
                println!("-i visualize");
                println!("-s <path> path to std");
                success = false;
            }
            _ => {
                file_given = true;
                if !process_dir(arg.clone(), &mut inputs) {
                    success = false;
                }
            }
        }
    }
    if !file_given {
        if success {
            eprintln!("no file given to compile");
        }
        success = false;
    }
    if success {
        if !process_dir(std_path, &mut inputs) {
            success = false;
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
