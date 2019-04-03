use crate::types::*;
use std::fmt::Debug;
use std::fmt::Error;
use std::fmt::Formatter;

#[derive(Debug, PartialEq, PartialOrd)]
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
    SExpression(IonSExpression),
}

#[derive(PartialEq, PartialOrd)]
pub struct IonDomValue {
    field: Option<IonSymbol>,
    annotations: Vec<IonSymbol>,
    value: IonValue,
}

impl IonDomValue {
    pub fn new(
        field: Option<IonSymbol>,
        annotations: Vec<IonSymbol>,
        value: IonValue,
    ) -> IonDomValue {
        IonDomValue {
            field,
            annotations,
            value,
        }
    }

    pub fn field(&self) -> &Option<IonSymbol> {
        &self.field
    }

    pub fn value(&self) -> &IonValue {
        &self.value
    }
}

impl Debug for IonDomValue {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let unknown = "$0";

        // Annotations
        for annotation in &self.annotations {
            match annotation.text() {
                Some(ref text) => write!(f, "'{}'::", text)?,
                None => write!(f, "$0::")?,
            };
        }

        use self::IonValue::*;
        match &self.value {
            Null(_) => write!(f, "null")?,
            Boolean(ref ion_boolean) => write!(f, "{}", ion_boolean.boolean_value())?,
            Integer(ref ion_integer) => write!(f, "{}", u64::from(*ion_integer))?,
            Float(ref ion_float) => write!(f, "{}", f64::from(ion_float.clone()))?,
            Decimal(ion_decimal) => write!(f, "{:?}", ion_decimal)?,
            Timestamp(ion_timestamp) => write!(f, "{:?}", ion_timestamp)?,
            Symbol(ref ion_symbol) => {
                match ion_symbol.text() {
                    Some(ref text) => write!(f, "'{:?}'", &text)?,
                    None => write!(f, "'{:?}'", unknown)?,
                };
            }
            String(ref ion_string) => write!(f, "{:?}", ion_string)?,
            //      Clob(IonClob),
            //      Blob(IonBlob),
            List(ref ion_list) => write!(f, "{:?}", ion_list)?,
            Struct(ref ion_struct) => write!(f, "{:?}", ion_struct)?,
            //      Struct(ref ion_struct) => {
            //        write!(f, "{{")?;
            //        for value in ion_struct.as_map().values() {
            //          write!(f, "{:?}, ", value)?;
            //        }
            //        write!(f, "}}")?;
            //      },
            //      SExpression(IonSExpression)
            v => write!(f, "{:?}", v)?,
        };
        Ok(())
    }
}
