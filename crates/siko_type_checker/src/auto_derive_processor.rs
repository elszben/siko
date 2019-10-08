use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::type_processor::process_type_signature;
use crate::type_store::ResolverContext;
use crate::type_store::TypeStore;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::Adt;
use siko_ir::types::AutoDeriveMode;
use siko_ir::types::Record;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignature;
use siko_ir::walker::walk_expr;
use siko_ir::walker::Visitor;
#[allow(unused)]
use siko_util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::dependency_processor::DependencyCollector;
use crate::dependency_processor::DependencyGroup;
use crate::dependency_processor::DependencyProcessor;

struct FunctionDependencyCollector<'a> {
    program: &'a Program,
    used_functions: BTreeSet<FunctionId>,
}

impl<'a> FunctionDependencyCollector<'a> {
    fn new(program: &'a Program) -> FunctionDependencyCollector<'a> {
        FunctionDependencyCollector {
            program: program,
            used_functions: BTreeSet::new(),
        }
    }
}

impl<'a> Visitor for FunctionDependencyCollector<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, _: ExprId, expr: &Expr) {
        match expr {
            Expr::StaticFunctionCall(id, _) => {
                self.used_functions.insert(*id);
            }
            _ => {}
        }
    }

    fn visit_pattern(&mut self, _: PatternId, _: &Pattern) {
        // do nothing
    }
}

pub struct TypedefDependencyProcessor<'a> {
    program: &'a Program,
    type_store: &'a mut TypeStore,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl<'a> TypedefDependencyProcessor<'a> {
    pub fn new(
        program: &'a Program,
        type_store: &'a mut TypeStore,
        adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
        variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) -> TypedefDependencyProcessor<'a> {
        TypedefDependencyProcessor {
            program: program,
            type_store: type_store,
            adt_type_info_map: adt_type_info_map,
            record_type_info_map: record_type_info_map,
            variant_type_info_map: variant_type_info_map,
        }
    }

    pub fn process_functions(&mut self) -> Vec<DependencyGroup<TypeDefId>> {
        let mut typedefs = Vec::new();
        for (id, typedef) in &self.program.typedefs.items {
            if let TypeDef::Record(record) = typedef {
                if record.fields.is_empty() {
                    //external
                    continue;
                }
            }
            let typedef = self.program.typedefs.get(id);
            match typedef {
                TypeDef::Adt(adt) => {
                    match &adt.auto_derive_mode {
                        AutoDeriveMode::Implicit => {
                            // derive specific set of classes,
                            // NYI
                            let mut context = ResolverContext::new();
                            let mut variants = Vec::new();
                            for index in 0..adt.variants.len() {
                                let key = (adt.id, index);
                                let variant_type_info = self
                                    .variant_type_info_map
                                    .get(&key)
                                    .expect("Variant type info not found");
                                for item_type in &variant_type_info.item_types {
                                    let variant_string =
                                        self.type_store.get_resolved_type_string_with_context(
                                            &item_type,
                                            &mut context,
                                        );
                                    variants.push(variant_string);
                                }
                            }
                            let adt_type_info = self
                                .adt_type_info_map
                                .get(&adt.id)
                                .expect("Adt type info not found");
                            let adt_type = self.type_store.get_resolved_type_string_with_context(
                                &adt_type_info.adt_type,
                                &mut context,
                            );
                            println!(
                                "IMPLICIT ADT {} {} TYPE:[{}] depends on {}",
                                adt.module,
                                adt.name,
                                adt_type,
                                variants[..].join(", ")
                            );
                        }
                        AutoDeriveMode::Explicit(derived_classes) => {
                            let mut arg_map = BTreeMap::new();
                            let mut variants = Vec::new();
                            for variant in &adt.variants {
                                for item in &variant.items {
                                    let mut handler = None;
                                    let var = process_type_signature(
                                        &mut self.type_store,
                                        &item.type_signature_id,
                                        self.program,
                                        &mut arg_map,
                                        &mut handler,
                                    );
                                    let variant_string =
                                        self.type_store.get_resolved_type_string(&var);
                                    variants.push(variant_string);
                                }
                            }
                            println!(
                                "ADT {} {} depends on {}",
                                adt.module,
                                adt.name,
                                variants[..].join(", ")
                            );
                        }
                    }
                }
                TypeDef::Record(record) => {
                    let mut arg_map = BTreeMap::new();
                    let mut fields = Vec::new();
                    for field in &record.fields {
                        let mut handler = None;
                        let var = process_type_signature(
                            &mut self.type_store,
                            &field.type_signature_id,
                            self.program,
                            &mut arg_map,
                            &mut handler,
                        );
                        let field_string = self.type_store.get_resolved_type_string(&var);
                        fields.push(field_string);
                    }
                    println!(
                        "Record {} {} depends on {}",
                        record.module,
                        record.name,
                        fields[..].join(", ")
                    );
                }
            }
        }
        let dep_processor = DependencyProcessor::new(typedefs);
        let ordered_function_groups = dep_processor.process_items(self);

        ordered_function_groups
    }
}

impl<'a> DependencyCollector<TypeDefId> for TypedefDependencyProcessor<'a> {
    fn collect(&self, typedef_id: TypeDefId) -> Vec<TypeDefId> {
        let mut deps = BTreeSet::new();
        deps.into_iter().collect()
    }
}
