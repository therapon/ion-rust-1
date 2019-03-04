use binary::ion_type_code::IonTypeCode;
use binary::ion_cursor::IonValueHeader;
use result::IonResult;

const MAX_NIBBLE_SIZE: u8 = 16;

lazy_static! {
    pub static ref SLOW_HEADERS: Vec<IonResult<Option<IonValueHeader>>>= {
        let mut headers = Vec::with_capacity(256);
        for byte_value in 0..=255 {
          headers.push(ion_value_header(byte_value));
        }
        headers
    };
}

fn ion_value_header(byte: u8) -> IonResult<Option<IonValueHeader>> {
  let (type_code, length_code) = nibbles_from_byte(byte);
  let ion_type_code = IonTypeCode::from(type_code)?;
  let ion_type = ion_type_code.as_type().ok();
  Ok(Some(IonValueHeader {
    ion_type,
    ion_type_code,
    length_code
  }))
}

pub fn nibbles_from_byte(byte: u8) -> (u8, u8) {
  let left = byte >> 4;
  let right = byte & 0b1111;
  (left, right)
}

pub fn byte_from_nibbles(left: u8, right: u8) -> u8 {
  assert!(left < MAX_NIBBLE_SIZE);
  assert!(right < MAX_NIBBLE_SIZE);
  let mut byte = 0u8;
  byte |= left << 4;
  byte |= 0b0000_1111 & right;
  byte
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_byte_from_nibbles() {
    assert_eq!(byte_from_nibbles(0b1111, 0b1111), 0b1111_1111);
    assert_eq!(byte_from_nibbles(0b0000, 0b0000), 0b0000_0000);
    assert_eq!(byte_from_nibbles(0b1111, 0b0000), 0b1111_0000);
    assert_eq!(byte_from_nibbles(0b0000, 0b1111), 0b0000_1111);
    assert_eq!(byte_from_nibbles(0b0011, 0b1100), 0b0011_1100);
    assert_eq!(byte_from_nibbles(0b1010, 0b0101), 0b1010_0101);
  }

  #[test]
  fn test_nibbles_from_byte() {
    assert_eq!(nibbles_from_byte(0b1111_1111), (0b1111, 0b1111));
    assert_eq!(nibbles_from_byte(0b0000_0000), (0b0000, 0b0000));
    assert_eq!(nibbles_from_byte(0b0000_1111), (0b0000, 0b1111));
    assert_eq!(nibbles_from_byte(0b1111_0000), (0b1111, 0b0000));
    assert_eq!(nibbles_from_byte(0b1010_1010), (0b1010, 0b1010));
    assert_eq!(nibbles_from_byte(0b0101_0101), (0b0101, 0b0101));
    assert_eq!(nibbles_from_byte(0b1001_1001), (0b1001, 0b1001));
  }
}