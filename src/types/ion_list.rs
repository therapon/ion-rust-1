use std::convert::From;
use types::*;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Error;
use lifeguard::RcRecycled;

#[derive(PartialEq, PartialOrd /*, Clone <-- RcRecycled */)]
pub struct IonList {
  values: RcRecycled<Vec<IonDomValue>>
}

impl IonList {
  //TODO: Better API
  pub fn values(&self) -> impl Iterator<Item=&IonDomValue> {
    self.values.iter()
  }
}

impl Debug for IonList {
  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
    write!(f, "[")?;
    for value in self.values() {
      write!(f, "{:?}, ", value)?;
    }
    write!(f, "]")?;
    Ok(())
  }
}

impl From<RcRecycled<Vec<IonDomValue>>> for IonList {
  fn from(values: RcRecycled<Vec<IonDomValue>>) -> Self {
    IonList {
      values
    }
  }
}