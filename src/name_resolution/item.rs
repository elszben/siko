use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::types::TypeDefId;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::function::FunctionId;

#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionId, IrFunctionId),
    Record(RecordId, TypeDefId),
    Adt(AdtId, TypeDefId),
}
