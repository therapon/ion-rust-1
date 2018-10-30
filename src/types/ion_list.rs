use std::convert::From;
use types::*;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct IonList {
  values: Vec<IonDomValue>
}

impl IonList {
  //TODO: Better API
  pub fn values(&self) -> &[IonDomValue] {
    &self.values
  }
}

impl From<Vec<IonDomValue>> for IonList {
  fn from(values: Vec<IonDomValue>) -> Self {
    IonList {
      values
    }
  }
}