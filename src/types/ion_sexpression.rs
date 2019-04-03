use crate::types::*;
use std::convert::From;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct IonSExpression {
    values: Vec<IonDomValue>,
}

impl IonSExpression {
    //TODO: Better API
    pub fn values(&self) -> &[IonDomValue] {
        &self.values
    }
}

impl From<Vec<IonDomValue>> for IonSExpression {
    fn from(values: Vec<IonDomValue>) -> Self {
        IonSExpression { values }
    }
}
