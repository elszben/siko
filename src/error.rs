use crate::file_manager::FileManager;
use crate::location_info::filepath::FilePath;
use crate::location_info::location::Location;
use crate::location_info::location_info::LocationInfo;
use crate::location_info::location_set::LocationSet;
use crate::name_resolution::error::InternalModuleConflict;
use crate::name_resolution::error::ResolverError;
use crate::typechecker::error::TypecheckError;
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
    let pipe = "|";
    let mut last_line = 0;
    for (line_index, ranges) in &location_set.lines {
        last_line = *line_index;
        if first {
            first = false;
            println!(
                "{}{}:{}",
                "-- ".blue(),
                location_set.file_path.path.green(),
                format!("{}", line_index + 1).green()
            );
            if *line_index != 0 {
                let line = &lines[*line_index - 1];
                println!("{} {}", pipe.blue(), line);
            }
        }
        let line = &lines[*line_index];
        let chars: Vec<_> = line.chars().collect();
        let first = s_from_range(&chars[..], 0, ranges[0].start);
        print!("{} {}", pipe.blue(), first);
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
    if last_line + 1 < lines.len() {
        let line = &lines[last_line + 1];
        println!("{} {}", pipe.blue(), line);
    }
}

#[derive(Debug)]
pub enum Error {
    IoError(IoError),
    LexerError(String, FilePath, Location),
    ParseError(String, FilePath, Location),
    ResolverError(Vec<ResolverError>),
    TypecheckError(Vec<TypecheckError>),
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

    pub fn typecheck_err(errors: Vec<TypecheckError>) -> Error {
        Error::TypecheckError(errors)
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
        let error = "ERROR:";
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
                                println!(
                                    "{} module name {} defined more than once",
                                    error.red(),
                                    name.yellow()
                                );
                                for id in ids.iter() {
                                    let location_set = location_info.get_item_location(id);
                                    print_location_set(file_manager, location_set);
                                }
                            }
                        }
                        ResolverError::ImportedModuleNotFound(errors) => {
                            for (name, id) in errors.iter() {
                                println!(
                                    "{} imported module {} does not exist",
                                    error.red(),
                                    name.yellow()
                                );
                                let location_set = location_info.get_item_location(id);
                                print_location_set(file_manager, location_set);
                            }
                        }
                        ResolverError::SymbolNotFoundInModule(name, id) => {
                            println!(
                                "{} imported symbol {} not found in module",
                                error.red(),
                                name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownTypeName(var_name, id) => {
                            println!("Unknown type name {}", var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::TypeArgumentConflict(args, id) => {
                            println!(
                                "Type argument(s) {} are not unique",
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ArgumentConflict(args, id) => {
                            println!("Argument(s) {} are not unique", format_list(args).yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::LambdaArgumentConflict(args, id) => {
                            println!(
                                "Lambda argument(s) {} are not unique",
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownFunction(var_name, id) => {
                            println!("Unknown function {}", var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::AmbiguousName(var_name, id) => {
                            println!("Ambiguous name {}", var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::FunctionTypeNameMismatch(n1, n2, id) => {
                            println!(
                                "Name mismatch in function type signature, {} != {}",
                                n1.yellow(),
                                n2.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnusedTypeArgument(args, id) => {
                            println!("Unused type argument(s): {}", format_list(args).yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::InternalModuleConflicts(module_conflicts) => {
                            for (module, conflicts) in module_conflicts {
                                for conflict in conflicts {
                                    match conflict {
                                        InternalModuleConflict::ItemConflict(name, locations) => {
                                            println!(
                                                "{} conflicting items named {} in module {}",
                                                error.red(),
                                                name.yellow(),
                                                module.yellow()
                                            );
                                            for id in locations {
                                                let location_set =
                                                    location_info.get_item_location(id);
                                                print_location_set(file_manager, location_set);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ResolverError::RecordTypeNameMismatch(record_name, type_name, id) => {
                            println!(
                                "{} type name {} does not match record name {}",
                                error.red(),
                                type_name.yellow(),
                                record_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::RecordFieldNotUnique(record_name, item_name, id) => {
                            println!(
                                "{} item name {} is not unique in record {}",
                                error.red(),
                                item_name.yellow(),
                                record_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::VariantNotUnique(adt_name, variant_name, id) => {
                            println!(
                                "{} variant name {} is not unique in type {}",
                                error.red(),
                                variant_name.yellow(),
                                adt_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ExportedEntityDoesNotExist(module_name, entity_name, id) => {
                            println!(
                                "{} exported entity {} does not exist in module {}",
                                error.red(),
                                entity_name.yellow(),
                                module_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::IncorrectNameInExportedTypeConstructor(
                            module_name,
                            type_name,
                            id,
                        ) => {
                            println!(
                                "{} exported type {} does not exist in module {}",
                                error.red(),
                                type_name.yellow(),
                                module_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ExportedRecordFieldDoesNotExist(
                            record_name,
                            field_name,
                            id,
                        ) => {
                            println!(
                                "{} exported record field name {} does not exist in record {}",
                                error.red(),
                                field_name.yellow(),
                                record_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ExportedAdtVariantDoesNotExist(
                            adt_name,
                            variant_name,
                            id,
                        ) => {
                            println!(
                                "{} exported type variant name {} does not exist in type {}",
                                error.red(),
                                variant_name.yellow(),
                                adt_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                    }
                }
            }
            Error::RuntimeError(err) => {
                println!("Error: {}", err);
            }
            Error::TypecheckError(errs) => {
                for err in errs {
                    match err {
                        TypecheckError::UntypedExternFunction(name, id) => {
                            println!(
                                "{} extern function {} must have a type signature",
                                error.red(),
                                name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::FunctionTypeDependencyLoop => {
                            println!("{} function type dependency loop detected", error.red());
                        }
                        TypecheckError::TooManyArguments(id, name, expected, found) => {
                            println!(
                                "{} too many arguments given for {}",
                                error.red(),
                                name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                            println!("Expected: {}", format!("{}", expected).yellow());
                            println!("Found: {}", format!("{}", found).yellow());
                        }
                        TypecheckError::TypeMismatch(id, expected, found) => {
                            println!("{} type mismatch in expression", error.red());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                            println!("Expected: {}", expected.yellow());
                            println!("Found:    {}", found.yellow());
                        }
                        TypecheckError::FunctionArgumentMismatch(id, args, func) => {
                            println!("{} invalid argument(s)", error.red());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                            println!("Argument(s):      {}", args.yellow());
                            println!("Function type:    {}", func.yellow());
                        }
                        TypecheckError::NotCallableType(id, ty) => {
                            println!(
                                "{} trying to call a non-callable type {}",
                                error.red(),
                                ty.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                    }
                }
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
