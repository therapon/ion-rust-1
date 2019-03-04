use std::io::Read;
use result::IonResult;

type IntStorage = i64;

#[derive(Debug)]
pub struct Int {
  size_in_bytes: usize,
  value: IntStorage
}

impl Int {
  pub fn read_int(data_source: &mut Read, length: usize) -> IonResult<Int> {
    if length == 0 {
      return Ok(Int {
        size_in_bytes: 0,
        value: 0
      });
    }

    let mut magnitude: IntStorage;

    let first_byte: i64 = i64::from(data_source.bytes().next().unwrap()?);
    let sign: IntStorage = if first_byte & 0b1000_0000 == 0 {1} else {-1};
    magnitude = first_byte & 0b0111_1111;

    //TODO: Read `length - 1` bytes at once instead of error handling each byte
    for byte in data_source.bytes().take(length - 1) {
      let byte = i64::from(byte?);
      magnitude <<= 8;
      magnitude |= byte;
    }
    Ok(Int {
      size_in_bytes: length,
      value: magnitude * sign
    })
  }

  #[inline(always)]
  pub fn value(&self) -> IntStorage {
    self.value
  }

  #[inline(always)]
  pub fn size_in_bytes(&self) -> usize {
    self.size_in_bytes
  }
}

#[cfg(test)]
mod tests {
  use super::Int;
  use std::io::Cursor;

  #[test]
  fn test_read_int() {
    let error_message = "Failed to read an Int from the provided cursor.";
    let data = &[0b0011_1100, 0b1000_0111, 0b1000_0001];
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 3);
    assert_eq!(int.value(), 3_966_849);

    let error_message = "Failed to read an Int from the provided cursor.";
    let data = &[0b1011_1100, 0b1000_0111, 0b1000_0001];
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 3);
    assert_eq!(int.value(), -3_966_849);

    let data = &[0b1000_0000]; // Negative zero
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 1);
    assert_eq!(int.value(), 0);

    let data = &[0b0000_0000]; // Positive zero
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 1);
    assert_eq!(int.value(), 0);

    let data = &[0b0111_1111, 0b1111_1111];
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 2);
    assert_eq!(int.value(), 32_767);

    let data = &[0b1111_1111, 0b1111_1111];
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 2);
    assert_eq!(int.value(), -32_767);

    let data = &[];
    let int = Int::read_int(
      &mut Cursor::new(data),
      data.len()
    ).expect(error_message);
    assert_eq!(int.size_in_bytes(), 0);
    assert_eq!(int.value(), 0);
  }
}