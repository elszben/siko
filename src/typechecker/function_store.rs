use super::constraint_store::ConstraintStore;
use super::function_type::FunctionType;
use super::function_type::FunctionTypeStore;
use super::type_store::TypeStore;
use crate::error::Error;
use crate::ir::FunctionId;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::mem;

#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub function_type: Option<FunctionType>,
    pub type_store: TypeStore,
    pub constraint_store: ConstraintStore,
}

impl FunctionInfo {
    pub fn new(name: String, function_type: Option<FunctionType>) -> FunctionInfo {
        FunctionInfo {
            name: name,
            function_type: function_type,
            type_store: TypeStore::new(),
            constraint_store: ConstraintStore::new(),
        }
    }

    pub fn dump(&self, id: FunctionId) {
        println!("Dump for {:?}", id);
        self.type_store.dump();
        self.constraint_store.dump();
    }

    pub fn get_used_functions(&self) -> &Vec<FunctionId> {
        &self.constraint_store.get_function_header().used_functions
    }
}

impl fmt::Display for FunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.function_type {
            Some(func_type) => write!(f, "{}", func_type),
            None => write!(f, "<>"),
        }
    }
}

#[derive(Debug)]
pub struct FunctionStore {
    functions: BTreeMap<FunctionId, FunctionInfo>,
}

impl FunctionStore {
    pub fn new() -> FunctionStore {
        FunctionStore {
            functions: BTreeMap::new(),
        }
    }

    pub fn dump(&self) {
        for (id, info) in &self.functions {
            info.dump(id.clone());
        }
    }

    pub fn get_function_info(&self, function_id: &FunctionId) -> &FunctionInfo {
        self.functions
            .get(function_id)
            .expect("Function is not registered")
    }

    pub fn get_function_info_mut(&mut self, function_id: &FunctionId) -> &mut FunctionInfo {
        self.functions
            .get_mut(function_id)
            .expect("Function is not registered")
    }

    pub fn add(&mut self, function_id: FunctionId, info: FunctionInfo) {
        self.functions.insert(function_id, info);
    }

    pub fn process(&mut self) -> Result<(), Error> {
        let mut function_type_store = FunctionTypeStore::new();

        let mut processed_function_set = BTreeSet::new();

        for (id, info) in &mut self.functions {
            if let Some(ty) = &info.constraint_store.get_function_header().function_type {
                function_type_store.add_function_type(id.clone(), ty.clone());
            }
        }

        loop {
            let mut progressed = false;
            for (id, info) in &mut self.functions {
                if processed_function_set.contains(id) {
                    continue;
                }
                let mut missing = false;
                for used_fn in info.get_used_functions() {
                    if !processed_function_set.contains(used_fn) {
                        if function_type_store.get_function_type(used_fn).is_none() {
                            missing = true;
                            break;
                        }
                    }
                }
                if missing {
                    continue;
                }
                processed_function_set.insert(id.clone());
                progressed = true;
                println!("---------------------------");
                println!("Before typecheck");
                info.dump(id.clone());
                info.constraint_store
                    .process(&mut info.type_store, &function_type_store)?;
                println!("After typecheck");
                info.constraint_store.check_header(&info.type_store)?;
                function_type_store.add_function_type(
                    id.clone(),
                    info.constraint_store
                        .get_function_header()
                        .function_type
                        .clone()
                        .expect("Missing functin type"),
                );
                info.dump(id.clone());
            }

            if !progressed {
                if processed_function_set.len() != self.functions.len() {
                    let err = Error::typecheck_err(format!("Function dependency loop"));
                    return Err(err);
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
