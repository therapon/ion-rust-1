use crate::types::*;
use lifeguard::RcRecycled;
use std::convert::From;
use std::fmt::Debug;
use std::fmt::Error;
use std::fmt::Formatter;

#[derive(PartialEq, PartialOrd)]
pub struct IonList {
    values: RcRecycled<Vec<IonDomValue>>,
}

impl IonList {
    //TODO: Better API
    pub fn values(&self) -> impl Iterator<Item = &IonDomValue> {
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
        IonList { values }
    }
}
