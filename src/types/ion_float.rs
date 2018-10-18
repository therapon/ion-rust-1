use std::convert::From;
use std::convert::AsRef;
use std::borrow::Cow;
use std::ops::Deref;

#[derive(Debug, PartialEq, Clone)]
pub struct IonFloat {
  value: f64
}

impl IonFloat {
  pub fn as_f64(&self) -> f64 {
    self.value
  }

  pub fn as_f32(&self) -> f32 {
    self.value as f32
  }
}

impl From<f64> for IonFloat {
  fn from(value: f64) -> Self {
    IonFloat {
      value
    }
  }
}