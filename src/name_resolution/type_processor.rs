use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature as IrTypeSignature;
use crate::ir::types::TypeSignatureId as IrTypeSignatureId;
use crate::location_info::item::LocationId;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::import::ImportedItemInfo;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::function::FunctionType as AstFunctionType;
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
        AstTypeSignature::Named(n, _) => {
            let name = n.get();
            match name.as_ref() {
                "Int" => IrTypeSignature::Int,
                "Bool" => IrTypeSignature::Bool,
                "String" => IrTypeSignature::String,
                _ => {
                    if let Some(index) = type_args.get(&name) {
                        used_type_args.insert(name.clone());
                        IrTypeSignature::TypeArgument(*index)
                    } else {
                        match module.imported_items.get(&name) {
                            Some(items) => {
                                let item = &items[0];
                                match item.item {
                                    Item::Adt(_, ir_typedef_id) => {
                                        IrTypeSignature::Named(ir_typedef_id, vec![])
                                    }
                                    Item::Record(_, ir_typedef_id) => {
                                        IrTypeSignature::Named(ir_typedef_id, vec![])
                                    }
                                    Item::Function(..) => unimplemented!(),
                                }
                            }
                            None => {
                                let error =
                                    ResolverError::UnknownTypeName(name.clone(), location_id);
                                errors.push(error);
                                return None;
                            }
                        }
                    }
                }
            }
        }
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
        AstTypeSignature::Function(items) => {
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
            IrTypeSignature::Function(item_ids)
        }
        AstTypeSignature::TypeArgument(_) => unimplemented!(),
    };
    let id = ir_program.get_type_signature_id();
    let type_info = TypeInfo::new(ir_type_signature, type_signature_id.clone());
    ir_program.add_type_signature(id, type_info);
    return Some(id);
}

pub fn process_type_signatures(
    original_type_args: &[String],
    type_signature_ids: &[TypeSignatureId],
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    location_id: LocationId,
    errors: &mut Vec<ResolverError>,
) -> Vec<Option<IrTypeSignatureId>> {
    let mut result = Vec::new();
    let mut type_args = BTreeMap::new();
    let mut conflicting_names = BTreeSet::new();
    for (index, type_arg) in original_type_args.iter().enumerate() {
        if type_args.insert(type_arg.clone(), index).is_some() {
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

    if !unused.is_empty() {
        let err = ResolverError::UnusedTypeArgument(unused, location_id);
        errors.push(err);
    }

    result
}
