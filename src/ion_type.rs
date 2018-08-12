use failure::Error;
use errors::*;
use ion_type_code::IonTypeCode;

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

impl IonType {
  pub fn from(ion_type_code: IonTypeCode) -> Result<IonType, IonError> {
    use self::IonTypeCode::*;
    let ion_type = match ion_type_code {
      Null => IonType::Null,
      Boolean => IonType::Boolean,
      PositiveInteger | NegativeInteger => IonType::Integer,
      Float => IonType::Float,
      Decimal => IonType::Decimal,
      Timestamp => IonType::Timestamp,
      Symbol => IonType::Symbol,
      String => IonType::String,
      Clob => IonType::Clob,
      Blob => IonType::Blob,
      List => IonType::List,
      SExpression => IonType::SExpression,
      Struct => IonType::Struct,
      _ => return decoding_error(
        format!(
          "Attempted to make an IonType from an invalid type code: {:?}",
          ion_type_code
        ).as_ref()
      )
    };
    Ok(ion_type)
  }
}