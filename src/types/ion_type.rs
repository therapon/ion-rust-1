#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum IonType {
  Null,
  Boolean,
  Integer,
  Float,
  Decimal,
  Timestamp,
  Symbol,
  String,
  Clob,
  Blob,
  List,
  SExpression,
  Struct,
}