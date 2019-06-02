use crate::ir::class::ClassId as IrClassId;
use crate::ir::class::ClassMemberId as IrClassMemberId;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::types::TypeDefId;
use crate::syntax::class::ClassId;
use crate::syntax::class::ClassMemberId;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;


#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionId, IrFunctionId),
    Record(RecordId, TypeDefId),
    Adt(AdtId, TypeDefId),
    Variant(AdtId, VariantId, TypeDefId, usize),
    Class(ClassId, IrClassId),
    ClassMember(ClassId, ClassMemberId, IrClassMemberId),
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
