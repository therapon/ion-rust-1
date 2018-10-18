use std::convert::From;
use std::ops::Deref;

// Borrowed byte array, requires no copying

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonClobRef<'a> {
  bytes: &'a [u8]
}

impl <'a> IonClobRef<'a> {
  pub fn bytes(&self) -> &[u8] {
    self.bytes
  }
}

impl <'a> From<&'a [u8]> for IonClobRef<'a> {
  fn from(bytes: &'a [u8]) -> Self {
    IonClobRef {
      bytes
    }
  }
}

// Owned byte vector, requires copying from the source buffer

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonClob {
  bytes: Vec<u8>
}

impl IonClob {
  pub fn bytes(&self) -> &[u8] {
    self.bytes.as_ref()
  }
}

impl <'a> From<&'a [u8]> for IonClob {
  fn from(byte_to_copy: &'a [u8]) -> Self {
    let mut bytes = Vec::with_capacity(byte_to_copy.len());
    bytes.copy_from_slice(byte_to_copy);
    IonClob {
      bytes
    }
  }
}

impl From<Vec<u8>> for IonClob {
  fn from(bytes: Vec<u8>) -> Self {
    IonClob {
      bytes
    }
  }
}