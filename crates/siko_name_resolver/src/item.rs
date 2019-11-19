use siko_ir::class::ClassId as IrClassId;
use siko_ir::class::ClassMemberId as IrClassMemberId;
use siko_ir::data::TypeDefId;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_syntax::class::ClassId;
use siko_syntax::data::AdtId;
use siko_syntax::data::RecordFieldId;
use siko_syntax::data::RecordId;
use siko_syntax::data::VariantId;
use siko_syntax::function::FunctionId;
use siko_syntax::function::FunctionTypeId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Item {
    Function(FunctionId, IrFunctionId),
    Record(RecordId, TypeDefId),
    Adt(AdtId, TypeDefId),
    Variant(AdtId, VariantId, TypeDefId, usize),
    Class(ClassId, IrClassId),
    ClassMember(ClassId, FunctionTypeId, IrClassMemberId),
}

#[derive(Debug, Clone)]
pub enum DataMember {
    RecordField(RecordField),
    Variant(Variant),
}

#[derive(Debug, Clone)]
pub struct RecordField {
    pub field_id: RecordFieldId,
    pub record_id: RecordId,
    pub ir_typedef_id: TypeDefId,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub variant_id: VariantId,
    pub adt_id: AdtId,
}
