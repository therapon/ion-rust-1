use std::convert::From;

// Borrowed byte array, requires no copying

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct IonBlobRef<'a> {
  bytes: &'a [u8]
}

impl <'a> IonBlobRef<'a> {
  pub fn bytes(&self) -> &[u8] {
    self.bytes
  }
}

impl <'a> From<&'a [u8]> for IonBlobRef<'a> {
  fn from(bytes: &'a [u8]) -> Self {
    IonBlobRef {
      bytes
    }
  }
}

// Owned byte vector, requires copying from the source buffer

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct IonBlob {
  bytes: Vec<u8>
}

impl IonBlob {
  pub fn bytes(&self) -> &[u8] {
    self.bytes.as_ref()
  }
}

impl <'a> From<&'a [u8]> for IonBlob {
  fn from(byte_to_copy: &'a [u8]) -> Self {
    let mut bytes = Vec::with_capacity(byte_to_copy.len());
    bytes.copy_from_slice(byte_to_copy);
    IonBlob {
      bytes
    }
  }
}

impl From<Vec<u8>> for IonBlob {
  fn from(bytes: Vec<u8>) -> Self {
    IonBlob {
      bytes
    }
  }
}

impl <'a> From<IonBlobRef<'a>> for IonBlob {
  fn from(blob_ref: IonBlobRef) -> Self {
    let mut bytes = Vec::with_capacity(blob_ref.bytes().len());
    bytes.extend(blob_ref.bytes().iter());
    IonBlob {
      bytes
    }
  }
}