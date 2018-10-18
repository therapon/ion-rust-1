use failure::Error;
use result::*;
use types::ion_type::IonType;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum IonTypeCode {
  Null, // 0
  Boolean, // 1
  PositiveInteger, // 2 for positive
  NegativeInteger, // 3 for negative
  Float, // 4
  Decimal, // 5
  Timestamp, // 6
  Symbol, // 7
  String, // 8
  Clob, // 9
  Blob, // 10
  List, // 11
  SExpression, // 12
  Struct, // 13
  Annotation, // 14
  Reserved // 15
}

impl IonTypeCode {
  pub fn as_type(&self) -> IonResult<IonType> {
    use self::IonTypeCode::*;
    let ion_type = match *self {
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
          self
        ).as_ref()
      )
    };
    Ok(ion_type)
  }

  pub fn from(type_code: u8) -> IonResult<IonTypeCode> {
    use self::IonTypeCode::*;
    let ion_type_code = match type_code {
      0 => Null,
      1 => Boolean,
      2 => PositiveInteger,
      3 => NegativeInteger,
      4 => Float,
      5 => Decimal,
      6 => Timestamp,
      7 => Symbol,
      8 => String,
      9 => Clob,
      10 => Blob,
      11 => List,
      12 => SExpression,
      13 => Struct,
      14 => Annotation,
      15 => Reserved,
      _ => {
        return decoding_error::<IonTypeCode>(
        format!("{:?} is not a valid header type code.", type_code).as_ref()
        );
      }
    };
    Ok(ion_type_code)
  }
}