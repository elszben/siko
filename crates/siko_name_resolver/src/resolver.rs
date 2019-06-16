use crate::environment::Environment;
use crate::error::Error;
use crate::error::ResolverError;
use crate::export_processor::process_exports;
use crate::expr_processor::process_expr;
use crate::import_processor::process_imports;
use crate::item::DataMember;
use crate::item::Item;
use crate::item::RecordField;
use crate::item::Variant;
use crate::lambda_helper::LambdaHelper;
use crate::module::Module;
use crate::type_arg_constraint_collector::TypeArgConstraintCollection;
use crate::type_arg_constraint_collector::TypeArgConstraintCollector;
use crate::type_processor::process_type_signatures;
use siko_ir::class::Class as IrClass;
use siko_ir::class::ClassId as IrClassId;
use siko_ir::class::ClassMember as IrClassMember;
use siko_ir::class::Instance as IrInstance;
use siko_ir::class::InstanceMember as IrInstanceMember;
use siko_ir::function::Function as IrFunction;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionInfo;
use siko_ir::function::RecordConstructorInfo;
use siko_ir::function::VariantConstructorInfo;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Adt;
use siko_ir::types::Record;
use siko_ir::types::RecordField as IrRecordField;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_ir::types::Variant as IrVariant;
use siko_ir::types::VariantItem;
use siko_location_info::item::LocationId;
use siko_syntax::class::ClassId as AstClassId;
use siko_syntax::class::Instance as AstInstance;
use siko_syntax::data::AdtId;
use siko_syntax::data::RecordId;

use siko_syntax::function::Function as AstFunction;
use siko_syntax::function::FunctionBody as AstFunctionBody;
use siko_syntax::function::FunctionType as AstFunctionType;
use siko_syntax::module::Module as AstModule;
use siko_syntax::program::Program;
use std::collections::BTreeMap;
use std::collections::BTreeSet;


#[derive(Debug)]
pub struct Resolver {
    modules: BTreeMap<String, Module>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            modules: BTreeMap::new(),
        }
    }

    fn register_module(
        &mut self,
        ast_module: &AstModule,
        modules: &mut BTreeMap<String, Vec<Module>>,
    ) {
        let module = Module::new(
            ast_module.id,
            ast_module.name.clone(),
            ast_module.location_id,
        );

        let mods = modules
            .entry(ast_module.name.clone())
            .or_insert_with(Vec::new);
        mods.push(module);
    }

    fn process_module_conflicts(
        &mut self,
        modules: BTreeMap<String, Vec<Module>>,
    ) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut module_conflicts = BTreeMap::new();

        for (name, modules) in modules.iter() {
            if modules.len() > 1 {
                let ids = modules.iter().map(|m| m.location_id).collect();
                module_conflicts.insert(name.clone(), ids);
            }
        }

        if !module_conflicts.is_empty() {
            let e = ResolverError::ModuleConflict(module_conflicts);
            errors.push(e);
        }

        if errors.is_empty() {
            for (name, mods) in modules {
                let modules: Vec<Module> = mods;
                self.modules
                    .insert(name, modules.into_iter().next().expect("Empty module set"));
            }
            Ok(())
        } else {
            return Err(Error::resolve_err(errors));
        }
    }

    fn process_items_and_types(
        &mut self,
        program: &Program,
        errors: &mut Vec<ResolverError>,
        ir_program: &mut IrProgram,
    ) {
        for (module_name, module) in &mut self.modules {
            let is_prelude = module_name == "Prelude";
            let ast_module = program.modules.get(&module.id);
            for record_id in &ast_module.records {
                let record = program.records.get(record_id);
                let ir_typedef_id = if is_prelude {
                    match record.name.as_ref() {
                        "Int" => ir_program.builtin_types.int_id,
                        "Bool" => ir_program.builtin_types.bool_id,
                        "String" => ir_program.builtin_types.string_id,
                        _ => ir_program.typedefs.get_id(),
                    }
                } else {
                    ir_program.typedefs.get_id()
                };
                let ir_ctor_id = ir_program.functions.get_id();
                let record_ctor_info = RecordConstructorInfo {
                    type_id: ir_typedef_id,
                };
                let ir_ctor_function = IrFunction {
                    id: ir_ctor_id,
                    arg_locations: record
                        .fields
                        .iter()
                        .map(|field_id| program.record_fields.get(field_id).location_id)
                        .collect(),
                    implicit_arg_count: 0,
                    info: FunctionInfo::RecordConstructor(record_ctor_info),
                };
                ir_program.functions.add_item(ir_ctor_id, ir_ctor_function);

                let ir_record = Record {
                    name: record.name.clone(),
                    id: ir_typedef_id,
                    type_args: (0..record.type_args.len()).collect(),
                    fields: Vec::new(),
                    constructor: ir_ctor_id,
                    location_id: record.location_id,
                };

                let typedef = TypeDef::Record(ir_record);
                ir_program.typedefs.add_item(ir_typedef_id, typedef);
                let item = Item::Record(*record_id, ir_typedef_id);
                module.add_item(record.name.clone(), item);
                for (index, field_id) in record.fields.iter().enumerate() {
                    let field = program.record_fields.get(field_id);
                    let record_field = RecordField {
                        field_id: field.id,
                        record_id: *record_id,
                        ir_typedef_id: ir_typedef_id,
                        index: index,
                    };
                    let member = DataMember::RecordField(record_field);
                    module.add_member(field.name.clone(), member);
                }
            }
            for adt_id in &ast_module.adts {
                let adt = program.adts.get(adt_id);
                let ir_typedef_id = ir_program.typedefs.get_id();
                let ir_adt = Adt {
                    name: adt.name.clone(),
                    id: ir_typedef_id,
                    type_args: (0..adt.type_args.len()).collect(),
                    variants: Vec::new(),
                };
                let typedef = TypeDef::Adt(ir_adt);
                ir_program.typedefs.add_item(ir_typedef_id, typedef);
                let item = Item::Adt(*adt_id, ir_typedef_id);
                module.add_item(adt.name.clone(), item);
                for (index, variant_id) in adt.variants.iter().enumerate() {
                    let ast_variant = program.variants.get(variant_id);
                    let item = Item::Variant(*adt_id, *variant_id, ir_typedef_id, index);
                    module.add_item(ast_variant.name.clone(), item);
                    let variant = Variant {
                        variant_id: *variant_id,
                        adt_id: *adt_id,
                    };
                    let member = DataMember::Variant(variant);
                    module.add_member(ast_variant.name.clone(), member);
                }
            }
            for function_id in &ast_module.functions {
                let function = program.functions.get(function_id);
                let ir_function_id = ir_program.functions.get_id();
                let item = Item::Function(function.id, ir_function_id);
                module.add_item(function.name.clone(), item);
            }
            for class_id in &ast_module.classes {
                let ir_class_id = ir_program.classes.get_id();
                let class = program.classes.get(class_id);
                let item = Item::Class(class.id, ir_class_id);
                module.add_item(class.name.clone(), item);
                let mut members = Vec::new();
                for member_id in &class.members {
                    let ir_class_member_id = ir_program.class_members.get_id();
                    let class_member = program.class_members.get(member_id);
                    let item = Item::ClassMember(class.id, *member_id, ir_class_member_id);
                    module.add_item(class_member.type_signature.name.clone(), item);
                    members.push(ir_class_member_id);
                }
                let ir_class = IrClass {
                    id: ir_class_id,
                    name: class.name.clone(),
                    members: members,
                    location_id: class.location_id,
                };
                ir_program.classes.add_item(ir_class_id, ir_class);
            }
        }

        for (_, module) in &self.modules {
            for (name, items) in &module.items {
                if items.len() > 1 {
                    let mut locations = Vec::new();
                    for item in items {
                        match item {
                            Item::Function(id, _) => {
                                let function = program.functions.get(id);
                                locations.push(function.location_id);
                            }
                            Item::Record(id, _) => {
                                let record = program.records.get(id);
                                locations.push(record.location_id);
                            }
                            Item::Adt(id, _) => {
                                let adt = program.adts.get(id);
                                locations.push(adt.location_id);
                            }
                            Item::Variant(_, id, _, _) => {
                                let variant = program.variants.get(id);
                                locations.push(variant.location_id);
                            }
                            Item::Class(id, _) => {
                                let class = program.classes.get(id);
                                locations.push(class.location_id);
                            }
                            Item::ClassMember(_, id, _) => {
                                let class_member = program.class_members.get(id);
                                locations.push(class_member.location_id);
                            }
                        }
                    }
                    let err = ResolverError::InternalModuleConflicts(
                        module.name.clone(),
                        name.clone(),
                        locations,
                    );
                    errors.push(err);
                }
            }
        }

        for (_, record) in &program.records.items {
            let mut field_names = BTreeSet::new();
            for field_id in &record.fields {
                let field = program.record_fields.get(field_id);
                if !field_names.insert(field.name.clone()) {
                    let err = ResolverError::RecordFieldNotUnique(
                        record.name.clone(),
                        field.name.clone(),
                        record.location_id,
                    );
                    errors.push(err);
                }
            }
        }

        for (_, adt) in &program.adts.items {
            let mut variant_names = BTreeSet::new();
            for variant_id in &adt.variants {
                let variant = program.variants.get(variant_id);
                if !variant_names.insert(variant.name.clone()) {
                    let err = ResolverError::VariantNotUnique(
                        adt.name.clone(),
                        variant.name.clone(),
                        adt.location_id,
                    );
                    errors.push(err);
                }
            }
        }
    }

    fn check_function_type_name(
        &self,
        function_type: &AstFunctionType,
        name: &String,
        errors: &mut Vec<ResolverError>,
    ) {
        if function_type.name != *name {
            let err = ResolverError::FunctionTypeNameMismatch(
                function_type.name.clone(),
                name.clone(),
                function_type.location_id,
            );
            errors.push(err);
        }
    }

    fn process_function_type(
        &self,
        function_type: &AstFunctionType,
        module: &Module,
        program: &Program,
        ir_program: &mut IrProgram,
        errors: &mut Vec<ResolverError>,
    ) -> (Option<TypeSignatureId>, TypeArgConstraintCollection) {
        let mut collector = TypeArgConstraintCollector::new();

        for (type_arg, _) in function_type.type_args.iter() {
            collector.add_empty(type_arg.clone());
        }

        for constraint in &function_type.constraints {
            if let Some(ir_class_id) = self.lookup_class(
                &constraint.class_name,
                constraint.location_id,
                module,
                errors,
            ) {
                collector.add_constraint(constraint.arg.clone(), ir_class_id);
            }
        }

        let type_args = collector.get_all_constraints();

        let result = process_type_signatures(
            type_args.clone(),
            &[function_type.type_signature_id],
            program,
            ir_program,
            module,
            function_type.location_id,
            errors,
            false,
            false,
        );

        (result[0], type_args)
    }

    fn process_function(
        &self,
        program: &Program,
        ir_program: &mut IrProgram,
        function: &AstFunction,
        ir_function_id: IrFunctionId,
        module: &Module,
        errors: &mut Vec<ResolverError>,
        type_signature_id: Option<TypeSignatureId>,
        type_args: &TypeArgConstraintCollection,
    ) {
        let mut body = None;

        if let AstFunctionBody::Expr(id) = function.body {
            let mut environment = Environment::new();
            let mut arg_names = BTreeSet::new();
            let mut conflicting_names = BTreeSet::new();
            for (index, arg) in function.args.iter().enumerate() {
                if !arg_names.insert(arg.0.clone()) {
                    conflicting_names.insert(arg.0.clone());
                }
                environment.add_arg(arg.0.clone(), ir_function_id, index);
            }
            if !conflicting_names.is_empty() {
                let err = ResolverError::ArgumentConflict(
                    conflicting_names.into_iter().collect(),
                    function.location_id.clone(),
                );
                errors.push(err);
            }
            let host_function = format!("{}/{}", module.name, function.name);
            let lambda_helper = LambdaHelper::new(
                0,
                host_function,
                LambdaHelper::new_counter(),
                ir_function_id,
                ir_function_id,
                None,
            );
            let body_id = process_expr(
                id,
                program,
                module,
                &mut environment,
                ir_program,
                errors,
                lambda_helper,
                type_args,
            );
            body = Some(body_id);
        }

        let named_info = NamedFunctionInfo {
            body: body,
            name: function.name.clone(),
            module: module.name.clone(),
            type_signature: type_signature_id,
            location_id: function.location_id,
        };

        let ir_function = IrFunction {
            id: ir_function_id,
            arg_locations: function.args.iter().map(|arg| arg.1).collect(),
            implicit_arg_count: 0,
            info: FunctionInfo::NamedFunction(named_info),
        };
        ir_program.functions.add_item(ir_function_id, ir_function);
    }

    fn process_adt(
        &self,
        program: &Program,
        ir_program: &mut IrProgram,
        adt_id: &AdtId,
        ir_typedef_id: TypeDefId,
        module: &Module,
        errors: &mut Vec<ResolverError>,
    ) {
        let adt = program.adts.get(adt_id);
        let mut type_signature_ids = Vec::new();
        for variant_id in &adt.variants {
            let variant = program.variants.get(&variant_id);
            type_signature_ids.push(variant.type_signature_id);
        }

        let mut collector = TypeArgConstraintCollector::new();

        for (type_arg, _) in adt.type_args.iter() {
            collector.add_empty(type_arg.clone());
        }

        let type_args = collector.get_all_constraints();

        let result = process_type_signatures(
            type_args,
            &type_signature_ids[..],
            program,
            ir_program,
            module,
            adt.location_id,
            errors,
            false,
            false,
        );

        if errors.is_empty() {
            let mut ir_variants = Vec::new();
            for (index, _) in adt.variants.iter().enumerate() {
                let ir_typesignature_id = result[index].expect("type signature missing");
                if let TypeSignature::Variant(name, items) = ir_program
                    .type_signatures
                    .get(&ir_typesignature_id)
                    .item
                    .clone()
                {
                    let items: Vec<_> = items
                        .iter()
                        .map(|i| VariantItem {
                            type_signature_id: *i,
                        })
                        .collect();
                    let ir_ctor_id = ir_program.functions.get_id();
                    let variant_ctor_info = VariantConstructorInfo {
                        type_id: ir_typedef_id,
                        index: index,
                    };
                    let ir_ctor_function = IrFunction {
                        id: ir_ctor_id,
                        arg_locations: items
                            .iter()
                            .map(|item| {
                                ir_program
                                    .type_signatures
                                    .get(&item.type_signature_id)
                                    .location_id
                            })
                            .collect(),
                        implicit_arg_count: 0,
                        info: FunctionInfo::VariantConstructor(variant_ctor_info),
                    };
                    ir_program.functions.add_item(ir_ctor_id, ir_ctor_function);

                    let ir_variant = IrVariant {
                        name: name.clone(),
                        items: items,
                        type_signature_id: ir_typesignature_id,
                        constructor: ir_ctor_id,
                    };

                    ir_variants.push(ir_variant);
                } else {
                    unreachable!()
                }
            }

            let ir_adt = ir_program.typedefs.get_mut(&ir_typedef_id).get_mut_adt();
            ir_adt.variants = ir_variants;
        }
    }

    fn process_record(
        &self,
        program: &Program,
        ir_program: &mut IrProgram,
        record_id: &RecordId,
        ir_typedef_id: TypeDefId,
        module: &Module,
        errors: &mut Vec<ResolverError>,
    ) {
        let record = program.records.get(record_id);
        let mut type_signature_ids = Vec::new();
        for field_id in &record.fields {
            let field = program.record_fields.get(field_id);
            type_signature_ids.push(field.type_signature_id);
        }

        let mut collector = TypeArgConstraintCollector::new();

        for (type_arg, _) in record.type_args.iter() {
            collector.add_empty(type_arg.clone());
        }

        let type_args = collector.get_all_constraints();

        let result = process_type_signatures(
            type_args,
            &type_signature_ids[..],
            program,
            ir_program,
            module,
            record.location_id,
            errors,
            record.external,
            false,
        );

        if errors.is_empty() {
            let mut ir_fields = Vec::new();
            for (index, field_id) in record.fields.iter().enumerate() {
                let field = program.record_fields.get(field_id);
                let ir_typesignature_id = result[index].expect("type signature missing");
                let ir_field = IrRecordField {
                    name: field.name.clone(),
                    type_signature_id: ir_typesignature_id,
                };
                ir_fields.push(ir_field);
            }

            let ir_record = ir_program.typedefs.get_mut(&ir_typedef_id).get_mut_record();
            ir_record.fields = ir_fields;
        }
    }

    fn lookup_class(
        &self,
        class_name: &String,
        location_id: LocationId,
        module: &Module,
        errors: &mut Vec<ResolverError>,
    ) -> Option<IrClassId> {
        match module.imported_items.get(class_name) {
            Some(items) => {
                if items.len() > 1 {
                    let err = ResolverError::AmbiguousName(class_name.clone(), location_id);
                    errors.push(err);
                    return None;
                }
                let item = &items[0];
                match item.item {
                    Item::Class(_, ir_class_id) => {
                        return Some(ir_class_id);
                    }
                    _ => {
                        let err = ResolverError::NotAClassName(class_name.clone(), location_id);
                        errors.push(err);
                    }
                }
            }
            None => {
                let err = ResolverError::NotAClassName(class_name.clone(), location_id);
                errors.push(err);
            }
        }
        None
    }

    fn process_class(
        &self,
        program: &Program,
        ir_program: &mut IrProgram,
        class_id: &AstClassId,
        ir_class_id: &IrClassId,
        module: &Module,
        errors: &mut Vec<ResolverError>,
    ) {
        let class = program.classes.get(class_id);

        let mut collector = TypeArgConstraintCollector::new();
        collector.add_constraint(class.arg.clone(), *ir_class_id);

        for constraint in &class.constraints {
            if class.arg != constraint.arg {
                let err = ResolverError::InvalidArgumentInTypeClassConstraint(
                    constraint.arg.clone(),
                    constraint.location_id,
                );
                errors.push(err);
            }
            if let Some(ir_class_id) = self.lookup_class(
                &constraint.class_name,
                constraint.location_id,
                module,
                errors,
            ) {
                collector.add_constraint(constraint.arg.clone(), ir_class_id);
            }
        }

        let ir_class = ir_program.classes.get(ir_class_id);
        let ir_class_member_ids = ir_class.members.clone();
        for (index, member_id) in class.members.iter().enumerate() {
            let class_member = program.class_members.get(member_id);
            let ir_class_member_id = ir_class_member_ids[index];
            let ty = &class_member.type_signature;
            let signature_type_args: BTreeSet<_> =
                ty.type_args.iter().map(|i| i.0.clone()).collect();
            if !signature_type_args.contains(&class.arg) || signature_type_args.len() != 1 {
                let err = ResolverError::ClassMemberTypeArgMismatch(
                    class.arg.clone(),
                    signature_type_args.into_iter().collect(),
                    ty.location_id,
                );
                errors.push(err);
                continue;
            }
            if !ty.constraints.is_empty() {
                for constraint in &ty.constraints {
                    let err = ResolverError::ExtraConstraintInClassMember(
                        class_member.type_signature.name.clone(),
                        constraint.location_id,
                    );
                    errors.push(err);
                }
                continue;
            }

            let type_args = collector.get_all_constraints();
            let result = process_type_signatures(
                type_args.clone(),
                &[ty.type_signature_id],
                program,
                ir_program,
                module,
                ty.location_id,
                errors,
                false,
                false,
            );

            if errors.is_empty() {
                let default_implementation =
                    if let Some(default_implementation) = &class_member.default_implementation {
                        let ir_function_id = ir_program.functions.get_id();
                        let function = program.functions.get(default_implementation);
                        let func_type = function
                            .func_type
                            .as_ref()
                            .expect("default class member has no type");
                        self.check_function_type_name(func_type, &function.name, errors);
                        self.process_function(
                            program,
                            ir_program,
                            &function,
                            ir_function_id,
                            module,
                            errors,
                            result[0],
                            &type_args,
                        );
                        Some(ir_function_id)
                    } else {
                        None
                    };
                let ir_class_member = IrClassMember {
                    id: ir_class_member_id,
                    class_id: *ir_class_id,
                    name: class_member.type_signature.name.clone(),
                    type_signature: result[0].expect("Type signature not found"),
                    default_implementation: default_implementation,
                    location_id: class_member.location_id,
                };
                ir_program
                    .class_members
                    .add_item(ir_class_member_id, ir_class_member);
            }
        }
    }

    fn process_instance(
        &self,
        instance: &AstInstance,
        program: &Program,
        ir_program: &mut IrProgram,
        module: &Module,
        errors: &mut Vec<ResolverError>,
    ) {
        let mut collector = TypeArgConstraintCollector::new();
        for constraint in &instance.constraints {
            if let Some(ir_class_id) = self.lookup_class(
                &constraint.class_name,
                constraint.location_id,
                module,
                errors,
            ) {
                collector.add_constraint(constraint.arg.clone(), ir_class_id);
            }
        }

        let ir_class_id =
            match self.lookup_class(&instance.class_name, instance.location_id, module, errors) {
                Some(ir_class_id) => ir_class_id,
                None => {
                    return;
                }
            };

        let type_args = collector.get_all_constraints();

        let result = process_type_signatures(
            type_args.clone(),
            &[instance.type_signature_id],
            program,
            ir_program,
            module,
            instance.location_id,
            errors,
            false,
            true,
        );

        let mut class_members = BTreeMap::new();

        let ir_class = ir_program.classes.get(&ir_class_id);

        for member_id in &ir_class.members {
            let ir_class_member = ir_program.class_members.get(&member_id);
            class_members.insert(
                ir_class_member.name.clone(),
                (
                    ir_class_member.default_implementation.clone(),
                    ir_class_member.id,
                    ir_class_member.type_signature,
                ),
            );
        }

        if let Some(type_signature) = result[0] {
            let id = ir_program.instances.get_id();

            let mut members = Vec::new();
            let mut implemented_members = BTreeSet::new();

            for member in &instance.members {
                let function = program.functions.get(&member.function_id);
                let member_name = &function.name;
                if let Some(class_member_info) = class_members.get(member_name) {
                    let ir_function_id = ir_program.functions.get_id();
                    let ir_instance_member = IrInstanceMember {
                        class_member_id: class_member_info.1,
                        function_id: ir_function_id,
                    };
                    members.push(ir_instance_member);
                    let class_member_type_signature_id = class_member_info.2;
                    let function = program.functions.get(&member.function_id);
                    self.process_function(
                        program,
                        ir_program,
                        &function,
                        ir_function_id,
                        module,
                        errors,
                        Some(class_member_type_signature_id),
                        &type_args,
                    );
                } else {
                    let err = ResolverError::NotAClassMember(
                        member_name.clone(),
                        instance.class_name.clone(),
                        function.location_id,
                    );
                    errors.push(err);
                }
                if !implemented_members.insert(member_name.clone()) {
                    let err = ResolverError::ClassMemberImplementedMultipleTimes(
                        member_name.clone(),
                        function.location_id,
                    );
                    errors.push(err);
                }
            }

            for (class_member, (default_impl, ir_class_member_id, _)) in &class_members {
                if !implemented_members.contains(class_member) {
                    match default_impl {
                        Some(default_impl) => {
                            let ir_instance_member = IrInstanceMember {
                                class_member_id: *ir_class_member_id,
                                function_id: *default_impl,
                            };
                            members.push(ir_instance_member);
                        }
                        None => {
                            if !implemented_members.contains(class_member) {
                                let err = ResolverError::MissingClassMemberInInstance(
                                    class_member.clone(),
                                    instance.class_name.clone(),
                                    instance.location_id,
                                );
                                errors.push(err);
                            }
                        }
                    }
                }
            }

            let ir_instance = IrInstance {
                id: id,
                class_id: ir_class_id,
                type_signature: type_signature,
                members: members,
                location_id: instance.location_id,
            };

            ir_program.instances.add_item(id, ir_instance);
        }
    }

    pub fn resolve(&mut self, program: &Program) -> Result<IrProgram, Error> {
        let mut errors = Vec::new();

        let mut modules = BTreeMap::new();

        for ast_module in program.modules.items.values() {
            self.register_module(ast_module, &mut modules);
        }

        self.process_module_conflicts(modules)?;

        let mut ir_program = IrProgram::new();

        self.process_items_and_types(program, &mut errors, &mut ir_program);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        process_exports(&mut self.modules, program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        process_imports(&mut self.modules, program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        for (_, module) in &self.modules {
            for (_, items) in &module.items {
                for item in items {
                    match item {
                        Item::Adt(ast_adt_id, ir_adt_id) => self.process_adt(
                            program,
                            &mut ir_program,
                            ast_adt_id,
                            *ir_adt_id,
                            module,
                            &mut errors,
                        ),
                        Item::Record(ast_record_id, ir_record_id) => self.process_record(
                            program,
                            &mut ir_program,
                            ast_record_id,
                            *ir_record_id,
                            module,
                            &mut errors,
                        ),
                        Item::Class(ast_class_id, ir_class_id) => self.process_class(
                            program,
                            &mut ir_program,
                            ast_class_id,
                            ir_class_id,
                            module,
                            &mut errors,
                        ),
                        _ => {}
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        for (_, module) in &self.modules {
            let ast_module = program.modules.get(&module.id);
            for instance_id in &ast_module.instances {
                let instance = program.instances.get(&instance_id);
                self.process_instance(instance, program, &mut ir_program, module, &mut errors);
            }
        }

        for (_, module) in &self.modules {
            for (_, items) in &module.items {
                for item in items {
                    match item {
                        Item::Function(ast_function_id, ir_function_id) => {
                            let function = program.functions.get(ast_function_id);
                            let (type_signature_id, type_args) =
                                if let Some(ty) = &function.func_type {
                                    self.check_function_type_name(ty, &function.name, &mut errors);
                                    self.process_function_type(
                                        ty,
                                        module,
                                        program,
                                        &mut ir_program,
                                        &mut errors,
                                    )
                                } else {
                                    (None, TypeArgConstraintCollection::new())
                                };
                            self.process_function(
                                program,
                                &mut ir_program,
                                &function,
                                *ir_function_id,
                                module,
                                &mut errors,
                                type_signature_id,
                                &type_args,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        Ok(ir_program)
    }
}
