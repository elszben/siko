use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeDef;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature as IrTypeSignature;
use crate::ir::types::TypeSignatureId as IrTypeSignatureId;
use crate::location_info::item::LocationId;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::name_resolution::type_arg_resolver::TypeArgResolver;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature as AstTypeSignature;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

fn process_named_type(
    name: &String,
    named_args: &Vec<TypeSignatureId>,
    location_id: LocationId,
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    type_arg_resolver: &mut TypeArgResolver,
    errors: &mut Vec<ResolverError>,
) -> Option<IrTypeSignatureId> {
    let ir_type_signature = match module.imported_items.get(name) {
        Some(items) => {
            if items.len() > 1 {
                let error = ResolverError::AmbiguousName(name.clone(), location_id);
                errors.push(error);
                return None;
            }
            let mut named_arg_ids = Vec::new();
            for named_arg in named_args {
                match process_type_signature(
                    named_arg,
                    program,
                    ir_program,
                    module,
                    type_arg_resolver,
                    errors,
                ) {
                    Some(id) => {
                        named_arg_ids.push(id);
                    }
                    None => {
                        return None;
                    }
                }
            }
            let item = &items[0];
            match item.item {
                Item::Adt(_, ir_typedef_id) => {
                    let ir_adt = ir_program
                        .typedefs
                        .get(&ir_typedef_id)
                        .expect("TypeDef not found");
                    match ir_adt {
                        TypeDef::Adt(adt) => {
                            if adt.type_args.len() != named_arg_ids.len() {
                                let err = ResolverError::IncorrectTypeArgumentCount(
                                    name.clone(),
                                    adt.type_args.len(),
                                    named_arg_ids.len(),
                                    location_id,
                                );
                                errors.push(err);
                                return None;
                            }
                            IrTypeSignature::Named(adt.name.clone(), ir_typedef_id, named_arg_ids)
                        }
                        TypeDef::Record(_) => unreachable!(),
                    }
                }
                Item::Record(_, ir_typedef_id) => {
                    let ir_record = ir_program
                        .typedefs
                        .get(&ir_typedef_id)
                        .expect("TypeDef not found");
                    match ir_record {
                        TypeDef::Adt(_) => unreachable!(),
                        TypeDef::Record(record) => {
                            if record.type_args.len() != named_arg_ids.len() {
                                let err = ResolverError::IncorrectTypeArgumentCount(
                                    name.clone(),
                                    record.type_args.len(),
                                    named_arg_ids.len(),
                                    location_id,
                                );
                                errors.push(err);
                                return None;
                            }
                            IrTypeSignature::Named(
                                record.name.clone(),
                                ir_typedef_id,
                                named_arg_ids,
                            )
                        }
                    }
                }
                Item::Function(..)
                | Item::Variant(..)
                | Item::ClassMember(..)
                | Item::Class(..) => {
                    let err = ResolverError::NameNotType(name.clone(), location_id);
                    errors.push(err);
                    return None;
                }
            }
        }
        None => {
            let error = ResolverError::UnknownTypeName(name.clone(), location_id);
            errors.push(error);
            return None;
        }
    };
    let id = ir_program.get_type_signature_id();
    let type_info = TypeInfo::new(ir_type_signature, location_id);
    ir_program.add_type_signature(id, type_info);
    return Some(id);
}

fn process_type_signature(
    type_signature_id: &TypeSignatureId,
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    type_arg_resolver: &mut TypeArgResolver,
    errors: &mut Vec<ResolverError>,
) -> Option<IrTypeSignatureId> {
    let type_signature = program.get_type_signature(type_signature_id);
    let location_id = program.get_type_signature_location(type_signature_id);
    let ir_type_signature = match type_signature {
        AstTypeSignature::Nothing => IrTypeSignature::Nothing,
        AstTypeSignature::TypeArg(name) => {
            if let Some(index) = type_arg_resolver.resolve_arg(name) {
                IrTypeSignature::TypeArgument(index, name.clone())
            } else {
                let error = ResolverError::UnknownTypeArg(name.clone(), location_id);
                errors.push(error);
                return None;
            }
        }
        AstTypeSignature::Variant(name, items) => {
            let mut item_ids = Vec::new();
            for item in items {
                match process_type_signature(
                    item,
                    program,
                    ir_program,
                    module,
                    type_arg_resolver,
                    errors,
                ) {
                    Some(id) => {
                        item_ids.push(id);
                    }
                    None => {
                        return None;
                    }
                }
            }
            IrTypeSignature::Variant(name.clone(), item_ids)
        }
        AstTypeSignature::Named(name, named_args) => match name.as_ref() {
            "Int" => IrTypeSignature::Int,
            "Bool" => IrTypeSignature::Bool,
            "String" => IrTypeSignature::String,
            _ => {
                return process_named_type(
                    name,
                    named_args,
                    location_id,
                    program,
                    ir_program,
                    module,
                    type_arg_resolver,
                    errors,
                );
            }
        },
        AstTypeSignature::Tuple(items) => {
            let mut item_ids = Vec::new();
            for item in items {
                match process_type_signature(
                    item,
                    program,
                    ir_program,
                    module,
                    type_arg_resolver,
                    errors,
                ) {
                    Some(id) => {
                        item_ids.push(id);
                    }
                    None => {}
                }
            }
            IrTypeSignature::Tuple(item_ids)
        }
        AstTypeSignature::Function(from, to) => {
            let ir_from = match process_type_signature(
                from,
                program,
                ir_program,
                module,
                type_arg_resolver,
                errors,
            ) {
                Some(id) => id,
                None => ir_program.get_type_signature_id(),
            };
            let ir_to = match process_type_signature(
                to,
                program,
                ir_program,
                module,
                type_arg_resolver,
                errors,
            ) {
                Some(id) => id,
                None => ir_program.get_type_signature_id(),
            };
            IrTypeSignature::Function(ir_from, ir_to)
        }
        AstTypeSignature::Wildcard => IrTypeSignature::Wildcard,
    };
    let id = ir_program.get_type_signature_id();
    let type_info = TypeInfo::new(ir_type_signature, location_id);
    ir_program.add_type_signature(id, type_info);
    return Some(id);
}

pub fn process_type_signatures(
    original_type_args: &[(String, LocationId)],
    type_signature_ids: &[TypeSignatureId],
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    location_id: LocationId,
    errors: &mut Vec<ResolverError>,
    external: bool,
    allow_implicit: bool,
) -> Vec<Option<IrTypeSignatureId>> {
    let mut type_arg_resolver = TypeArgResolver::new(allow_implicit);
    let mut result = Vec::new();
    let mut type_arg_names = BTreeSet::new();
    let mut conflicting_names = BTreeSet::new();
    for (type_arg, _) in original_type_args {
        if !allow_implicit {
            type_arg_resolver.add_explicit(type_arg.clone());
        }
        if !type_arg_names.insert(type_arg.clone()) {
            conflicting_names.insert(type_arg.clone());
        }
    }
    if !conflicting_names.is_empty() {
        let error = ResolverError::TypeArgumentConflict(
            conflicting_names.iter().cloned().collect(),
            location_id,
        );
        errors.push(error);
    }
    for type_signature_id in type_signature_ids {
        let id = process_type_signature(
            &type_signature_id,
            program,
            ir_program,
            module,
            &mut type_arg_resolver,
            errors,
        );
        result.push(id);
    }

    let mut unused = Vec::new();
    for (type_arg, _) in original_type_args {
        if !type_arg_resolver.contains(type_arg) {
            unused.push(type_arg.clone());
        }
    }

    if !unused.is_empty() && !external {
        let err = ResolverError::UnusedTypeArgument(unused, location_id);
        errors.push(err);
    }

    result
}
