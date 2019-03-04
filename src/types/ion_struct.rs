use std::convert::From;
use types::*;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;
use std::cmp::Ordering;
use lifeguard::RcRecycled;

pub struct IonStruct {
  //TODO: Symbol -> IonDomValue?

  //FIXME: HashMap doesn't work because it's not PartialOrd
  // TreeMap was very slow due to lots of allocations
//  values: HashMap<String, IonDomValue>

    values: RcRecycled<Vec<IonDomValue>>
}

impl Debug for IonStruct {
  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
    write!(f, "{{")?;
    for value in self.values() {
      let field = value.field().as_ref().expect("Missing field_id in struct field.");
      let field_name = field.text().expect("Missing field name in struct field.");
      write!(f, "{:?}: {:?}, ", field_name, value)?;
    }
    write!(f, "}}")?;
    Ok(())
  }
}

impl PartialEq for IonStruct {
  fn eq(&self, _other: &IonStruct) -> bool {
    false
  }
}

impl PartialOrd for IonStruct {
  fn partial_cmp(&self, _other: &IonStruct) -> Option<Ordering> {
    None
  }
}

//impl Clone for IonStruct {
//  fn clone(&self) -> Self {
//    IonStruct {
//      values: self.values.pool
//    }
//  }
//}

//TODO: IonString everywhere?

impl IonStruct {
  //TODO: Better API
  pub fn values(&self) -> impl Iterator<Item=&IonDomValue> {
    self.values.iter()//.map()
  }
}

impl From<RcRecycled<Vec<IonDomValue>>> for IonStruct {
//  impl From<HashMap<String, IonDomValue>> for IonStruct {
//  fn from(values: HashMap<String, IonDomValue>) -> Self {
fn from(values: RcRecycled<Vec<IonDomValue>>) -> Self {
    IonStruct {
      values
    }
  }
}