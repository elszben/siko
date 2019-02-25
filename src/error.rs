use crate::file_manager::FileManager;
use crate::location_info::filepath::FilePath;
use crate::location_info::location::Location;
use crate::location_info::location_info::LocationInfo;
use crate::location_info::location_set::LocationSet;
use crate::name_resolution::error::ResolverError;
use crate::util::format_list;
use colored::*;
use std::convert::From;
use std::io::Error as IoError;

fn s_from_range(chars: &[char], start: usize, end: usize) -> String {
    let subs = &chars[start..end];
    let s: String = subs.iter().collect();
    s
}

fn print_location_set(file_manager: &FileManager, location_set: &LocationSet) {
    let input = file_manager.content(&location_set.file_path);
    let lines: Vec<_> = input.lines().collect();
    let mut first = true;
    for (line_index, ranges) in &location_set.lines {
        if first {
            first = false;
            println!(
                "--{}:{}",
                location_set.file_path.path.green(),
                format!("{}", line_index).green()
            );
        }
        let line = &lines[*line_index];
        let chars: Vec<_> = line.chars().collect();
        let first = s_from_range(&chars[..], 0, ranges[0].start);
        print!("{}", first);
        for (index, range) in ranges.iter().enumerate() {
            let s = s_from_range(&chars[..], range.start, range.end);
            print!("{}", s.yellow());
            if index < ranges.len() - 1 {
                let s = s_from_range(&chars[..], range.end, ranges[index + 1].start);
                print!("{}", s);
            }
        }
        let last = s_from_range(&chars[..], ranges[ranges.len() - 1].end, chars.len());
        println!("{}", last);
    }
}

#[derive(Debug)]
pub enum Error {
    IoError(IoError),
    LexerError(String, FilePath, Location),
    ParseError(String, FilePath, Location),
    ResolverError(Vec<ResolverError>),
    TypeCheckError(String),
    RuntimeError(String),
}

impl Error {
    pub fn lexer_err(s: String, file_path: FilePath, location: Location) -> Error {
        Error::LexerError(s, file_path, location)
    }

    pub fn parse_err(s: String, file_path: FilePath, location: Location) -> Error {
        Error::ParseError(s, file_path, location)
    }

    pub fn resolve_err(errors: Vec<ResolverError>) -> Error {
        Error::ResolverError(errors)
    }

    pub fn typecheck_err(s: String) -> Error {
        Error::TypeCheckError(s)
    }

    pub fn runtime_err(s: String) -> Error {
        Error::RuntimeError(s)
    }
    fn report_location(file_manager: &FileManager, file_path: &FilePath, location: &Location) {
        let input = file_manager.content(file_path);
        let lines: Vec<_> = input.lines().collect();
        println!(
            "--{}:{}",
            file_path.path.green(),
            format!("{}", location.line).green()
        );
        let line = &lines[location.line];
        let chars: Vec<_> = line.chars().collect();
        let first = s_from_range(&chars[..], 0, location.span.start);
        print!("{}", first);
        let s = s_from_range(&chars[..], location.span.start, location.span.end);
        print!("{}", s.red());
        let last = s_from_range(&chars[..], location.span.end, chars.len());
        println!("{}", last);
    }

    fn report_error_base(
        msg: &str,
        file_manager: &FileManager,
        file_path: &FilePath,
        location: &Location,
    ) {
        println!("{}", msg);
        Error::report_location(file_manager, file_path, location);
    }

    pub fn report_error(&self, file_manager: &FileManager, location_info: &LocationInfo) {
        match self {
            Error::LexerError(msg, file_path, location) => {
                Error::report_error_base(msg, file_manager, file_path, location);
            }
            Error::ParseError(msg, file_path, location) => {
                Error::report_error_base(msg, file_manager, file_path, location);
            }
            Error::ResolverError(errs) => {
                for err in errs {
                    match err {
                        ResolverError::ModuleConflict(errors) => {
                            for (name, ids) in errors.iter() {
                                println!("Module name {} defined more than once", name.yellow());
                                for id in ids.iter() {
                                    let location_set = location_info.get_module_location(id);
                                    print_location_set(file_manager, location_set);
                                    //Error::report_location(file_manager, location);
                                }
                            }
                        }
                        ResolverError::ImportedModuleNotFound(errors) => {
                            for (name, id) in errors.iter() {
                                println!("Imported module {} does not exist", name.yellow());
                                let location_set = location_info.get_import_location(id);
                                print_location_set(file_manager, location_set);
                            }
                        }
                        ResolverError::SymbolNotFoundInModule(name, id) => {
                            println!("Imported symbol {} not found in module", name.yellow());
                            let location_set = location_info.get_import_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownTypeName(var_name, id) => {
                            println!("Unknown type name {}", var_name.yellow());
                            let location_set = location_info.get_type_signature_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::TypeArgumentConflict(args, id) => {
                            println!(
                                "Type argument(s) {} are not unique",
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_type_signature_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ArgumentConflict(args, id) => {
                            println!("Argument(s) {} are not unique", format_list(args).yellow());
                            let location_set = location_info.get_function_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::LambdaArgumentConflict(args, id) => {
                            println!(
                                "Lambda argument(s) {} are not unique",
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_expr_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownFunction(var_name, id) => {
                            println!("Unknown function {}", var_name.yellow());
                            let location_set = location_info.get_expr_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::AmbiguousName(var_name, id) => {
                            println!("Ambiguous name {}", var_name.yellow());
                            let location_set = location_info.get_expr_location(id);
                            print_location_set(file_manager, location_set);
                        } /*
                          ResolverError::FunctionConflictInModule(errors) => {
                              for ((name, module), locations) in errors.iter() {
                                  println!(
                                      "Function name {} defined more than once in module {}",
                                      name, module
                                  );
                                  println!("Locations:");
                                  for location in locations.iter() {
                                      Error::report_location(file_manager, location);
                                  }
                              }
                          }


                          ResolverError::ImportedSymbolConflict(module, conflicts) => {
                              for (symbol, modules) in conflicts.iter() {
                                  let modules: Vec<_> = modules.iter().map(|s| s.clone()).collect();
                                  println!(
                                      "Symbol {} imported from {} in",
                                      symbol,
                                      modules.join(", ")
                                  );
                              }
                              Error::report_location(file_manager, &module.location());
                          }
                          ResolverError::ArgumentConflict(func_name, location) => {
                              println!("Arguments for {} are not unique", func_name);
                              Error::report_location(file_manager, location);
                          }



                          */
                    }
                }
            }
            Error::RuntimeError(err) => {
                println!("Error: {}", err);
            }
            Error::TypeCheckError(err) => {
                println!("Error: {}", err);
            }
            _ => unimplemented!(),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Error {
        Error::IoError(e)
    }
}
