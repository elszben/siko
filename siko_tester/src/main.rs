use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;
use std::collections::BTreeSet;

fn process_dir(arg: String, inputs: &mut Vec<(PathBuf, String)>) -> bool {
    let path = Path::new(&arg);
    if !path.exists() {
        let path_str = format!("{}", path.display());
        eprintln!("ERROR: path {} does not exist", path_str);
        return false;
    }
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                if let Some(filename) = entry.path().file_name() {
                    if filename == "main.sk" {
                        let parent_dir = entry.path().parent().expect("Parent dir not found");
                        let testcase_name = parent_dir.file_name().expect("TC name not found");
                        let source_path = PathBuf::from(parent_dir);
                        inputs.push((source_path, testcase_name.to_str().expect("Name print failed").to_string()));
                    }
                }
            }
        }
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
    let mut success_count = 0;
    let mut fail_count = 0;
    let mut failed_tcs = BTreeSet::new();
    for (s, tc_name) in success_files {
        println!("TC-S: {}", tc_name);
        let status = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg(s.clone())
            .status()
            .expect("failed to execute process");
        if status.success() {
            success_count += 1;
        } else {
            fail_count += 1;
            failed_tcs.insert(tc_name.clone());
        }
        //println!("Compiling {}", s.display());
        let status = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg("-c")
            .arg(format!("{}/{}.rs", comp_dir, tc_name))
            .arg(s.clone())
            .status()
            .expect("failed to execute process");
        if status.success() {
            success_count += 1;
        } else {
            fail_count += 1;
            failed_tcs.insert(tc_name.clone());
        }
    }
    for (f, tc_name) in fail_files {
        println!("TC-F: {}", tc_name);
        let output = Command::new(sikoc.clone())
            .arg("-s")
            .arg(siko_std.clone())
            .arg(f.clone())
            .output()
            .expect("failed to execute process");
        let output_filename = format!("{}/{}.output", f.display(), tc_name);
        fs::write(output_filename, output.stderr).expect("output file write failed");
        if !output.status.success() {
            success_count += 1;
        } else {
            fail_count += 1;
            failed_tcs.insert(tc_name.clone());
        }
    }

    if !failed_tcs.is_empty() {
        println!("{} TCs failed.", failed_tcs.len());
        for tc in failed_tcs {
            println!("- {}", tc);
        }
        false
    }
    else
    {
        true
    }
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();

    let success = process_args(args);

    if !success {
        std::process::exit(1);
    }
}
