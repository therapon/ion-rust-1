use std::io::Read;
use errors::IonError;

type UIntStorage = u64;

#[derive(Debug)]
pub struct UInt {
  size_in_bytes: usize,
  value: UIntStorage
}

impl UInt {
  pub fn read_uint(data_source: &mut Read, length: usize) -> Result<UInt, IonError> {
    let mut magnitude: UIntStorage = 0;
    for (_i, byte) in data_source.bytes().take(length).enumerate() {
      let byte = (byte?) as UIntStorage;
      magnitude = magnitude << 8;
      magnitude = magnitude | byte;
    }
    Ok(UInt {
      size_in_bytes: length,
      value: magnitude
    })
  }

  #[inline(always)]
  pub fn value(&self) -> UIntStorage{
    self.value
  }

  #[inline(always)]
  pub fn size_in_bytes(&self) -> usize {
    self.size_in_bytes
  }
}

#[cfg(test)]
mod tests {
  use super::UInt;
  use std::io::Cursor;

  #[test]
  fn test_read_uint() {
    let error_message = "Failed to read a UInt from the provided cursor.";
    let data = &[0b0011_1100, 0b1000_0111, 0b1000_0001];
    let uint = UInt::read_uint(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(uint.size_in_bytes(), 3);
    assert_eq!(uint.value(), 3_966_849);

    let data = &[0b1000_0000];
    let uint = UInt::read_uint(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(uint.size_in_bytes(), 1);
    assert_eq!(uint.value(), 128);

    let data = &[0b0111_1111, 0b1111_1111];
    let uint = UInt::read_uint(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(uint.size_in_bytes(), 2);
    assert_eq!(uint.value(), 32_767);
  }
}