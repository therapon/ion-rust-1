use types::*;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum IonValue {
  Null(IonNull),
  Boolean(IonBoolean),
  Integer(IonInteger),
  Float(IonFloat),
  Decimal(IonDecimal),
  Timestamp(IonTimestamp),
  Symbol(IonSymbol),
  String(IonString),
  Clob(IonClob),
  Blob(IonBlob),
  List(IonList),
  Struct(IonStruct),
  SExpression(IonSExpression)
}

#[derive(PartialEq, PartialOrd, Clone)]
pub struct IonDomValue {
  field_id: Option<IonSymbol>,
  annotations: Vec<IonSymbol>,
  value: IonValue
}

impl IonDomValue {
  pub fn new(field_id: Option<IonSymbol>,
             annotations: Vec<IonSymbol>,
             value: IonValue) -> IonDomValue {
    IonDomValue {
      field_id,
      annotations,
      value
    }
  }
}

impl Debug for IonDomValue {
  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
    let unknown: IonString = "$0".into();

    // Field ID
    match self.field_id {
      Some(ref symbol) => {
        match symbol.text() {
          Some(ref text) => write!(f, "'{}': ", text.as_ref())?,
          None => write!(f, "{:?}: ", unknown)?,
        };
      },
      None => {}
    };
    // Annotations
    for annotation in &self.annotations {
      match annotation.text() {
        Some(ref text) => write!(f, "'{}'::", text.as_ref())?,
        None => write!(f, "$0::")?,
      };
    }

    use self::IonValue::*;
    match self.value {
      Null(_) => write!(f, "null")?,
      Boolean(ref ion_boolean) => write!(f, "{}", ion_boolean.boolean_value())?,
      Integer(ref ion_integer) => write!(f, "{}", u64::from(ion_integer.clone()))?,
      Float(ref ion_float) => write!(f, "{}", f64::from(ion_float.clone()))?,
//      Decimal(ion_decimal) => write!(f, "{}", ),
//      Timestamp(IonTimestamp),
      Symbol(ref ion_symbol) => {
        match ion_symbol.text() {
          Some(ref text) => write!(f, "'{:?}'", &text)?,
          None => write!(f, "'{:?}'", unknown)?,
        };
      },
      String(ref ion_string) => write!(f, "\"{}\"", ion_string.as_ref())?,
//      Clob(IonClob),
//      Blob(IonBlob),
//      List(IonList),
      Struct(ref ion_struct) => {
        write!(f, "{{")?;
        for value in ion_struct.as_map().values() {
          write!(f, "{:?}, ", value)?;
        }
        write!(f, "}}")?;
      },
//      SExpression(IonSExpression)
      ref v @ _ => write!(f, "{:?}", v)?
    };
    Ok(())
  }
}