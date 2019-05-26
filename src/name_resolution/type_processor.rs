use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeDef;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature as IrTypeSignature;
use crate::ir::types::TypeSignatureId as IrTypeSignatureId;
use crate::location_info::item::LocationId;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature as AstTypeSignature;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;


fn process_type_signature(
    type_signature_id: &TypeSignatureId,
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    type_args: &BTreeMap<String, usize>,
    errors: &mut Vec<ResolverError>,
    used_type_args: &mut BTreeSet<String>,
) -> Option<IrTypeSignatureId> {
    let type_signature = program.get_type_signature(type_signature_id);
    let location_id = program.get_type_signature_location(type_signature_id);
    let ir_type_signature = match type_signature {
        AstTypeSignature::Nothing => IrTypeSignature::Nothing,
        AstTypeSignature::Variant(name, items) => {
            let mut item_ids = Vec::new();
            for item in items {
                match process_type_signature(
                    item,
                    program,
                    ir_program,
                    module,
                    type_args,
                    errors,
                    used_type_args,
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
                if let Some(index) = type_args.get(name) {
                    used_type_args.insert(name.clone());
                    IrTypeSignature::TypeArgument(*index, name.clone())
                } else {
                    match module.imported_items.get(name) {
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
                                    type_args,
                                    errors,
                                    used_type_args,
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
                                            IrTypeSignature::Named(
                                                adt.name.clone(),
                                                ir_typedef_id,
                                                named_arg_ids,
                                            )
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
                                Item::Function(..) | Item::Variant(..) => {
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
                    }
                }
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
                    type_args,
                    errors,
                    used_type_args,
                ) {
                    Some(id) => {
                        item_ids.push(id);
                    }
                    None => {
                        return None;
                    }
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
                type_args,
                errors,
                used_type_args,
            ) {
                Some(id) => id,
                None => {
                    return None;
                }
            };
            let ir_to = match process_type_signature(
                to,
                program,
                ir_program,
                module,
                type_args,
                errors,
                used_type_args,
            ) {
                Some(id) => id,
                None => {
                    return None;
                }
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
) -> Vec<Option<IrTypeSignatureId>> {
    let mut result = Vec::new();
    let mut type_args = BTreeMap::new();
    let mut conflicting_names = BTreeSet::new();
    for (index, type_arg) in original_type_args.iter().enumerate() {
        if type_args.insert(type_arg.0.clone(), index).is_some() {
            conflicting_names.insert(type_arg.0.clone());
        }
    }
    if !conflicting_names.is_empty() {
        let error = ResolverError::TypeArgumentConflict(
            conflicting_names.iter().cloned().collect(),
            location_id,
        );
        errors.push(error);
    }

    let mut used_type_args = BTreeSet::new();

    for type_signature_id in type_signature_ids {
        let id = process_type_signature(
            &type_signature_id,
            program,
            ir_program,
            module,
            &type_args,
            errors,
            &mut used_type_args,
        );
        result.push(id);
    }

    let mut unused = Vec::new();
    for type_arg in type_args.keys() {
        if !used_type_args.contains(type_arg) {
            unused.push(type_arg.clone());
        }
    }

    if !unused.is_empty() && !external {
        let err = ResolverError::UnusedTypeArgument(unused, location_id);
        errors.push(err);
    }

    result
}
