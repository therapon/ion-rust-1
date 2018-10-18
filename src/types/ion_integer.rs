use std::convert::From;

//TODO: This value's internal representation should be flexible enough to support BigInteger-style
// values. For now, we're just starting with i64.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct IonInteger {
  value: i64
}

impl From<u64> for IonInteger {
  fn from(value: u64) -> Self {
    IonInteger {
      value: value as i64
    }
  }
}

impl From<i64> for IonInteger {
  fn from(value: i64) -> Self {
    IonInteger {
      value
    }
  }
}

impl From<usize> for IonInteger {
  fn from(value: usize) -> Self {
    IonInteger {
      value: value as i64
    }
  }
}

impl From<isize> for IonInteger {
  fn from(value: isize) -> Self {
    IonInteger {
      value: value as i64
    }
  }
}

impl Into<u64> for IonInteger {
  fn into(self) -> u64 {
    self.value as u64
  }
}

impl Into<i64> for IonInteger {
  fn into(self) -> i64 {
    self.value as i64
  }
}

impl Into<isize> for IonInteger {
  fn into(self) -> isize {
    self.value as isize
  }
}

impl Into<usize> for IonInteger {
  fn into(self) -> usize {
    self.value as usize
  }
}