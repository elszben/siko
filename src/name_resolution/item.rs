use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::function::FunctionId;

#[derive(Debug)]
pub enum Item {
    Function(FunctionId),
    Record(RecordId),
    Adt(AdtId),
}
