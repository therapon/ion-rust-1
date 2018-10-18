use std::convert::From;
use std::ops::Deref;
use ion_type::IonType;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonNull {
  ion_type: IonType
}

impl IonNull {
  fn ion_type(&self) -> IonType {
    self.ion_type
  }
}

impl From<bool> for IonNull {
  fn from(ion_type: IonType) -> Self {
    IonNull {
      ion_type
    }
  }
}