use std::convert::From;
use types::*;

#[derive(Debug, PartialEq, PartialOrd, /*Clone <-- RcRecycled */)]
pub struct IonSExpression {
  values: Vec<IonDomValue>
}

impl IonSExpression {
  //TODO: Better API
  pub fn values(&self) -> &[IonDomValue] {
    &self.values
  }
}

impl From<Vec<IonDomValue>> for IonSExpression {
  fn from(values: Vec<IonDomValue>) -> Self {
    IonSExpression {
      values
    }
  }
}