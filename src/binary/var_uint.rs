use std::io::Read;

use result::IonResult;
use std::io::BufRead;
use std::io::BufReader;

// TODO: variable size storage
type VarUIntStorage = usize;

#[derive(Debug)]
pub struct VarUInt {
  size_in_bytes: usize,
  value: VarUIntStorage
}

impl VarUInt {

  pub fn read_var_uint<R: Read>(data_source: &mut BufReader<R>) -> IonResult<VarUInt> {
    let mut number_of_bytes = 0;
    let mut magnitude: VarUIntStorage = 0;

    let mut number_of_buffered_bytes;
    let mut number_of_bytes_consumed = 0;

    'reading: loop {
      { // Extra scope to drop the reference to `data_source` that's held by `buffer` when we're done.
        let buffer = data_source.buffer();
        number_of_buffered_bytes = buffer.len();

        for byte in buffer {
          let byte = *byte;
          number_of_bytes += 1;
          let lower_seven = 0b0111_1111 & byte;
          let lower_seven = lower_seven as VarUIntStorage;
          magnitude = magnitude << 7; // Shifts 0 to 0 in the first iteration
          magnitude = magnitude | lower_seven;
          if byte >= 0b1000_0000 {
            break 'reading;
          }
        }
      }

      data_source.consume(number_of_buffered_bytes);
      number_of_bytes_consumed += number_of_buffered_bytes;
      let _ = data_source.fill_buf()?;
    }

    data_source.consume(number_of_bytes - number_of_bytes_consumed);

    Ok(VarUInt {
      size_in_bytes: number_of_bytes,
      value: magnitude
    })
  }

  #[inline(always)]
  pub fn value(&self) -> usize {
    self.value
  }

  #[inline(always)]
  pub fn size_in_bytes(&self) -> VarUIntStorage {
    self.size_in_bytes
  }
}

#[cfg(test)]
mod tests {
  use super::VarUInt;
  use std::io::Cursor;
  use result::IonError;
  use std::io::BufReader;

  #[test]
  fn test_read_var_uint() {
    let error_message = "Failed to read a VarUInt from the provided cursor.";
    let varuint = VarUInt::read_var_uint(&mut BufReader::new(
      Cursor::new(&[0b0111_1001, 0b0000_1111, 0b1000_0001])
    )).expect(error_message);
    assert_eq!(varuint.size_in_bytes(), 3);
    assert_eq!(varuint.value(), 1_984_385);

    let varuint = VarUInt::read_var_uint(&mut BufReader::with_capacity(
      1,
      Cursor::new(&[0b0111_1001, 0b0000_1111, 0b1000_0001]),
    )).expect(error_message);
    assert_eq!(varuint.size_in_bytes(), 3);
    assert_eq!(varuint.value(), 1_984_385);

    let varuint = VarUInt::read_var_uint(&mut BufReader::new(
      Cursor::new(&[0b1000_0000])
    )).expect(error_message);
    assert_eq!(varuint.size_in_bytes(), 1);
    assert_eq!(varuint.value(), 0);
    let varuint = VarUInt::read_var_uint(&mut BufReader::new(
      Cursor::new(&[0b0111_1111, 0b1111_1111])
    )).expect(error_message);
    assert_eq!(varuint.size_in_bytes(), 2);
    assert_eq!(varuint.value(), 16_383);
  }
}