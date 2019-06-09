use crate::error::ResolverError;
use crate::item::Item;
use crate::module::Module;
use crate::type_arg_resolver::TypeArgResolver;
use siko_ir::class::ClassId;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::TypeSignature as IrTypeSignature;
use siko_ir::types::TypeSignatureId as IrTypeSignatureId;
use siko_location_info::item::ItemInfo;
use siko_location_info::item::LocationId;
use siko_syntax::program::Program;
use siko_syntax::types::TypeSignature as AstTypeSignature;
use siko_syntax::types::TypeSignatureId;
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
                    let ir_adt = ir_program.typedefs.get(&ir_typedef_id).get_adt();
                    if ir_adt.type_args.len() != named_arg_ids.len() {
                        let err = ResolverError::IncorrectTypeArgumentCount(
                            name.clone(),
                            ir_adt.type_args.len(),
                            named_arg_ids.len(),
                            location_id,
                        );
                        errors.push(err);
                        return None;
                    }
                    IrTypeSignature::Named(ir_adt.name.clone(), ir_typedef_id, named_arg_ids)
                }
                Item::Record(_, ir_typedef_id) => {
                    let ir_record = ir_program.typedefs.get(&ir_typedef_id).get_record();

                    if ir_record.type_args.len() != named_arg_ids.len() {
                        let err = ResolverError::IncorrectTypeArgumentCount(
                            name.clone(),
                            ir_record.type_args.len(),
                            named_arg_ids.len(),
                            location_id,
                        );
                        errors.push(err);
                        return None;
                    }
                    IrTypeSignature::Named(ir_record.name.clone(), ir_typedef_id, named_arg_ids)
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
    let id = ir_program.type_signatures.get_id();
    let type_info = ItemInfo::new(ir_type_signature, location_id);
    ir_program.type_signatures.add_item(id, type_info);
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
    let info = program.type_signatures.get(type_signature_id);
    let type_signature = &info.item;
    let location_id = info.location_id;
    let ir_type_signature = match type_signature {
        AstTypeSignature::Nothing => IrTypeSignature::Nothing,
        AstTypeSignature::TypeArg(name) => {
            if let Some(info) = type_arg_resolver.resolve_arg(name) {
                IrTypeSignature::TypeArgument(info.index, name.clone(), info.constraints)
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
                None => ir_program.type_signatures.get_id(),
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
                None => ir_program.type_signatures.get_id(),
            };
            IrTypeSignature::Function(ir_from, ir_to)
        }
        AstTypeSignature::Wildcard => IrTypeSignature::Wildcard,
    };
    let id = ir_program.type_signatures.get_id();
    let type_info = ItemInfo::new(ir_type_signature, location_id);
    ir_program.type_signatures.add_item(id, type_info);
    return Some(id);
}

pub fn process_type_signatures(
    original_type_args: Vec<(String, Vec<ClassId>)>,
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
    for (type_arg, constraints) in original_type_args {
        if !allow_implicit {
            type_arg_resolver.add_explicit(type_arg.clone(), constraints);
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
    for type_arg in type_arg_names.iter() {
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
