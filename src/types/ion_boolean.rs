use std::convert::From;
use std::ops::Deref;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonBoolean {
  value: bool
}

impl IonBoolean {
  fn is_true(&self) -> bool {
    self.value
  }

  fn is_false(&self) -> bool {
    !self.value
  }

  fn boolean_value(&self) -> bool {
    self.value
  }
}

impl From<bool> for IonBoolean {
  fn from(value: bool) -> Self {
    IonBoolean {
      value
    }
  }
}