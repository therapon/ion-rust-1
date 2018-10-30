use std::convert::From;
use chrono::prelude::*;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct IonTimestamp {
  datetime: DateTime<FixedOffset>
}

//impl IonTimestamp {
//  pub fn as_datetime(&self) -> DateTime<Utc> {
//    self.datetime.clone()
//  }
//}

impl From<DateTime<FixedOffset>> for IonTimestamp {
  fn from(datetime: DateTime<FixedOffset>) -> Self {
    IonTimestamp {
      datetime
    }
  }
}

impl Into<DateTime<FixedOffset>> for IonTimestamp {
  fn into(self) -> DateTime<FixedOffset> {
    self.datetime
  }
}