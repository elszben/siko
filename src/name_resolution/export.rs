use crate::syntax::data::AdtId;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;

#[derive(Debug)]
pub enum ExportedItem {
    Function(FunctionId),
    Record(RecordId),
    DataConstructor(VariantId),
    RecordField(RecordFieldId),
}

#[derive(Debug)]
pub enum ExportedType {
    Record(RecordId),
    TypeConstructor(AdtId),
}
