use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

fn process_dir(arg: String, inputs: &mut Vec<PathBuf>) -> bool {
    let path = Path::new(&arg);
    if !path.exists() {
        let path_str = format!("{}", path.display());
        eprintln!("ERROR: path {} does not exist", path_str);
        return false;
    }
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry.unwrap();
            if let Some(ext) = entry.path().extension() {
                if ext == "sk" {
                    let input = PathBuf::from(entry.path());
                    inputs.push(input);
                }
            }
        }
    } else if path.is_file() {
        let input = PathBuf::from(path);
        inputs.push(input);
    }
    true
}

fn print_usage() {
    println!("Usage:");
    println!("SikoTester SIKOC SIKO_STD COMP_DIR SUCCESS_DIRFAIL_DIR");
}

fn process_args(args: Vec<String>) -> bool {
    if args.len() != 5 {
        print_usage();
        return false;
    }
    let sikoc = args[0].clone();
    let siko_std = args[1].clone();
    let comp_dir = args[2].clone();
    let success_dir = args[3].clone();
    let fail_dir = args[4].clone();
    let mut success_files = Vec::new();
    process_dir(success_dir, &mut success_files);
    let mut fail_files = Vec::new();
    process_dir(fail_dir, &mut fail_files);
    for (index, s) in success_files.iter().enumerate() {
        println!("Testing for SUCCESS: {}", s.display());
        let status = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg(s.clone())
            .status()
            .expect("failed to execute process");
        assert!(status.success());
        //println!("Compiling {}", s.display());
        let status = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg("-c")
            .arg(format!("{}/comp_test{}.rs", comp_dir, index))
            .arg(s.clone())
            .status()
            .expect("failed to execute process");
        assert!(status.success());
    }
    for f in fail_files {
        println!("Testing for FAIL: {}", f.display());
        let output = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg(f.clone())
            .output()
            .expect("failed to execute process");
        let output_filename = format!("{}.output", f.display());
        fs::write(output_filename, output.stderr).expect("output file write failed");
        assert!(!output.status.success());
    }
    true
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    let success = process_args(args);

    if !success {
        std::process::exit(1);
    }
}
