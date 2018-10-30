//use std::collections::HashMap;
use std::collections::BTreeMap;
use std::convert::From;
use types::*;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct IonStruct {
  //TODO: Symbol -> IonDomValue?
  values: BTreeMap<String, IonDomValue>
}

//TODO: IonString everywhere?

impl IonStruct {
  //TODO: Better API
  pub fn as_map(&self) -> &BTreeMap<String, IonDomValue> {
    &self.values
  }
}

impl From<BTreeMap<String, IonDomValue>> for IonStruct {
  fn from(values: BTreeMap<String, IonDomValue>) -> Self {
    IonStruct {
      values
    }
  }
}