use colored::*;
use siko_location_info::error_context::ErrorContext;
use siko_location_info::file_manager::FileManager;
use siko_location_info::filepath::FilePath;
use siko_location_info::item::LocationId;
use siko_location_info::location::Location;
use siko_location_info::location_set::LocationSet;
use siko_name_resolver::error::Error as ResolverErrorContainer;
use siko_name_resolver::error::ResolverError;
use siko_parser::error::LexerError;
use siko_parser::error::ParseError;
use siko_type_checker::error::Error as TypecheckErrorContainer;
use siko_type_checker::error::TypecheckError;
use siko_util::format_list;
use std::cmp;
use std::convert::From;
use std::io::Error as IoError;

fn s_from_range(chars: &[char], start: usize, end: usize) -> String {
    let start = cmp::min(start, end);
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
    LexerError(Vec<LexerError>),
    ParseError(ParseError),
    ResolverError(ResolverErrorContainer),
    TypecheckError(TypecheckErrorContainer),
    RuntimeError(String, LocationId),
}

impl Error {
    fn report_location(file_manager: &FileManager, file_path: &FilePath, location: &Location) {
        let input = file_manager.content(file_path);
        let lines: Vec<_> = input.lines().collect();
        println!(
            "--{}:{}",
            file_path.path.green(),
            format!("{}", location.line + 1).green()
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
        let error = "ERROR:";
        println!("{} {}", error.red(), msg);
        Error::report_location(file_manager, file_path, location);
    }

    pub fn report_error(&self, context: &ErrorContext) {
        let file_manager = context.file_manager;
        let location_info = context.location_info;
        let error = "ERROR:";
        match self {
            Error::LexerError(errors) => {
                for err in errors {
                    match err {
                        LexerError::General(msg, file_path, location) => {
                            Error::report_error_base(msg, file_manager, file_path, location);
                        }
                        LexerError::UnsupportedCharacter(c, location) => {
                            Error::report_error_base(
                                &format!(
                                    "{} unsupported character {}",
                                    error.red(),
                                    format!("{}", c).yellow()
                                ),
                                file_manager,
                                &location.file_path,
                                &location.location,
                            );
                        }
                    }
                }
            }
            Error::ParseError(err) => {
                Error::report_error_base(&err.msg, file_manager, &err.file_path, &err.location);
            }
            Error::ResolverError(errs) => {
                for err in &errs.errors {
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
                        ResolverError::ImportedModuleNotFound(name, id) => {
                            println!(
                                "{} imported module {} does not exist",
                                error.red(),
                                name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownTypeName(var_name, id) => {
                            println!("{} unknown type name {}", error.red(), var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownTypeArg(var_name, id) => {
                            println!(
                                "{} unknown type argument {}",
                                error.red(),
                                var_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::TypeArgumentConflict(args, id) => {
                            println!(
                                "{} type argument(s) {} are not unique",
                                error.red(),
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ArgumentConflict(args, id) => {
                            println!(
                                "{} argument(s) {} are not unique",
                                error.red(),
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::LambdaArgumentConflict(args, id) => {
                            println!(
                                "{} lambda argument(s) {} are not unique",
                                error.red(),
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownFunction(var_name, id) => {
                            println!("{} unknown function {}", error.red(), var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::AmbiguousName(var_name, id) => {
                            println!("{} ambiguous name {}", error.red(), var_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::FunctionTypeNameMismatch(n1, n2, id) => {
                            println!(
                                "{} name mismatch in function type signature, {} != {}",
                                error.red(),
                                n1.yellow(),
                                n2.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnusedTypeArgument(args, id) => {
                            println!(
                                "{} unused type argument(s): {}",
                                error.red(),
                                format_list(args).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::InternalModuleConflicts(module_name, name, locations) => {
                            println!(
                                "{} conflicting items named {} in module {}",
                                error.red(),
                                name.yellow(),
                                module_name.yellow()
                            );
                            for id in locations {
                                let location_set = location_info.get_item_location(id);
                                print_location_set(file_manager, location_set);
                            }
                        }
                        ResolverError::RecordFieldNotUnique(record_name, item_name, id) => {
                            println!(
                                "{} field name {} is not unique in record {}",
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
                        ResolverError::ExportNoMatch(module_name, entity_name, id) => {
                            println!(
                                "{} item {} does not export anything in module {}",
                                error.red(),
                                entity_name.yellow(),
                                module_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::ImportNoMatch(module_name, entity_name, id) => {
                            println!(
                                "{} item {} does not import anything from module {}",
                                error.red(),
                                entity_name.yellow(),
                                module_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::IncorrectTypeArgumentCount(
                            type_name,
                            expected,
                            found,
                            id,
                        ) => {
                            println!(
                                "{} incorrect type argument count for type {}",
                                error.red(),
                                type_name.yellow(),
                            );
                            let expected = format!("{}", expected);
                            let found = format!("{}", found);
                            println!("Expected: {}", expected.yellow());
                            println!("Found:    {}", found.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NameNotType(name, id) => {
                            println!("{} name is not a type {}", error.red(), name.yellow(),);
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnusedHiddenItem(hidden_item, module_name, id) => {
                            println!(
                                "{} hidden item {} does not hide anything from module {}",
                                error.red(),
                                hidden_item.yellow(),
                                module_name.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::UnknownFieldName(field_name, id) => {
                            println!("{} unknown field name {}", error.red(), field_name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NotIrrefutablePattern(id) => {
                            println!("{} not irrefutable pattern", error.red(),);
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NotRecordType(name, id) => {
                            println!("{} {} is not a record type", error.red(), name.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NoSuchField(record, field_name, id) => {
                            println!(
                                "{} there is no field named {} in {}",
                                error.red(),
                                field_name.yellow(),
                                record.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::MissingFields(missing_fields, id) => {
                            println!(
                                "{} missing initialization of the following field(s): {}",
                                error.red(),
                                format_list(missing_fields).yellow(),
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::FieldsInitializedMultipleTimes(
                            fields_initialized_twice,
                            id,
                        ) => {
                            println!(
                                "{} the following field(s) are initialized multiple times: {}",
                                error.red(),
                                format_list(fields_initialized_twice).yellow(),
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NoRecordFoundWithFields(fields, id) => {
                            println!(
                                "{} no record found that has all the following field(s): {}",
                                error.red(),
                                format_list(fields).yellow(),
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::NotAClassName(name, id) => {
                            println!(
                                "{} {} is not an imported type class",
                                error.red(),
                                name.yellow(),
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        ResolverError::InvalidArgumentInTypeClassConstraint(arg, id) => {
                            println!(
                                "{} type class constraint argument {} is unknown type argument",
                                error.red(),
                                arg.yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                    }
                }
            }
            Error::RuntimeError(err, id) => {
                println!("{} {}", error.red(), err);
                let location_set = location_info.get_item_location(id);
                print_location_set(file_manager, location_set);
            }
            Error::TypecheckError(errs) => {
                for err in &errs.errors {
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
                        TypecheckError::TypeMismatch(id, expected, found) => {
                            println!("{} type mismatch in expression", error.red());
                            println!("Expected: {}", expected.yellow());
                            println!("Found:    {}", found.yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::FunctionArgumentMismatch(id, args, func) => {
                            println!("{} invalid argument(s)", error.red());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                            println!("Argument(s):      {}", args.yellow());
                            println!("Function type:    {}", func.yellow());
                        }
                        TypecheckError::FunctionArgAndSignatureMismatch(
                            name,
                            arg_count,
                            signature_arg_count,
                            id,
                        ) => {
                            println!(
                                "{} function type signature for {} does not match its argument count",
                                error.red(),
                                name.yellow()
                            );
                            println!(
                                "Arguments:                      {}",
                                format!("{}", arg_count).yellow()
                            );
                            println!(
                                "Arguments in type signature:    {}",
                                format!("{}", signature_arg_count).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::MainNotFound => {
                            println!(
                                "{} {} function in module {} not found",
                                error.red(),
                                "main".yellow(),
                                "Main".yellow()
                            );
                        }
                        TypecheckError::RecursiveType(id) => {
                            println!("{} function type is recursive", error.red());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::InvalidFormatString(id) => {
                            println!("{} invalid format string", error.red());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::AmbiguousFieldAccess(id, records) => {
                            println!(
                                "{} ambiguous field access, record type could be {}",
                                error.red(),
                                format_list(records).yellow()
                            );
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::InvalidVariantPattern(id, name, expected, found) => {
                            println!(
                                "{} invalid {} variant pattern, argument count mismatch",
                                error.red(),
                                name.yellow()
                            );
                            println!("Expected:      {}", format!("{}", expected).yellow());
                            println!("Found:         {}", format!("{}", found).yellow());
                            let location_set = location_info.get_item_location(id);
                            print_location_set(file_manager, location_set);
                        }
                        TypecheckError::InvalidRecordPattern(id, name, expected, found) => {
                            println!(
                                "{} invalid {} record pattern, argument count mismatch",
                                error.red(),
                                name.yellow()
                            );
                            println!("Expected:      {}", format!("{}", expected).yellow());
                            println!("Found:         {}", format!("{}", found).yellow());
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

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Error {
        Error::ParseError(e)
    }
}

impl From<ResolverErrorContainer> for Error {
    fn from(e: ResolverErrorContainer) -> Error {
        Error::ResolverError(e)
    }
}

impl From<TypecheckErrorContainer> for Error {
    fn from(e: TypecheckErrorContainer) -> Error {
        Error::TypecheckError(e)
    }
}