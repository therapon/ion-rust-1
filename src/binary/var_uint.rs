use std::mem;

use crate::result::IonResult;
use std::io::BufRead;
use std::io::Write;

const SIZE_OF_U64: usize = mem::size_of::<u64>();
// TODO: variable size storage
type VarUIntStorage = usize;

#[derive(Debug)]
pub struct VarUInt {
    size_in_bytes: usize,
    value: VarUIntStorage,
}

impl VarUInt {
    pub fn read_var_uint<R: BufRead>(data_source: &mut R) -> IonResult<VarUInt> {
        let mut number_of_bytes = 0;
        let mut magnitude: VarUIntStorage = 0;

        let mut number_of_buffered_bytes;
        let mut number_of_bytes_consumed = 0;

        'reading: loop {
            {
                // Extra scope to drop the reference to `data_source` that's held by `buffer` when we're done.
                let buffer = data_source.fill_buf()?;
                number_of_buffered_bytes = buffer.len();

                for byte in buffer {
                    let byte = *byte;
                    number_of_bytes += 1;
                    let lower_seven = 0b0111_1111 & byte;
                    let lower_seven = lower_seven as VarUIntStorage;
                    magnitude <<= 7; // Shifts 0 to 0 in the first iteration
                    magnitude |= lower_seven;
                    if byte >= 0b1000_0000 {
                        break 'reading;
                    }
                }
            }

            data_source.consume(number_of_buffered_bytes);
            number_of_bytes_consumed += number_of_buffered_bytes;
        }

        data_source.consume(number_of_bytes - number_of_bytes_consumed);

        Ok(VarUInt {
            size_in_bytes: number_of_bytes,
            value: magnitude,
        })
    }

    pub fn write_var_uint<W: Write>(sink: &mut W, mut value: u64) -> IonResult<()> {
        // The last byte has the 'finished' flag set from the beginning.
        let mut output_buffer: [u8; SIZE_OF_U64] = [0, 0, 0, 0, 0, 0, 0, 0b1000_0000];
        let mut bytes_written = 0;

        for byte in output_buffer.iter_mut().rev() {
            *byte |= 0b0111_1111 & (value as u8);
            bytes_written += 1;
            value >>= 7; // Shift value over so the next bytes are ready to be written
            if value == 0 {
                break;
            }
        }

        let first_occupied_byte = output_buffer.len() - bytes_written;
        sink.write_all(&output_buffer[first_occupied_byte..])?;
        Ok(())
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

    use std::io::BufReader;

    #[test]
    fn test_read_var_uint() {
        let error_message = "Failed to read a VarUInt from the provided cursor.";
        let varuint = VarUInt::read_var_uint(&mut BufReader::new(Cursor::new(&[
            0b0111_1001,
            0b0000_1111,
            0b1000_0001,
        ])))
        .expect(error_message);
        assert_eq!(varuint.size_in_bytes(), 3);
        assert_eq!(varuint.value(), 1_984_385);

        let varuint = VarUInt::read_var_uint(&mut BufReader::with_capacity(
            1,
            Cursor::new(&[0b0111_1001, 0b0000_1111, 0b1000_0001]),
        ))
        .expect(error_message);
        assert_eq!(varuint.size_in_bytes(), 3);
        assert_eq!(varuint.value(), 1_984_385);

        let varuint = VarUInt::read_var_uint(&mut BufReader::new(Cursor::new(&[0b1000_0000])))
            .expect(error_message);
        assert_eq!(varuint.size_in_bytes(), 1);
        assert_eq!(varuint.value(), 0);
        let varuint = VarUInt::read_var_uint(&mut BufReader::new(Cursor::new(&[
            0b0111_1111,
            0b1111_1111,
        ])))
        .expect(error_message);
        assert_eq!(varuint.size_in_bytes(), 2);
        assert_eq!(varuint.value(), 16_383);
    }

    #[test]
    fn test_write_var_uint_1() {
        let value: u64 = 0b0111_1001__0000_1111__1000_0001;
        let expected_bytes = &[0b0_000_0011, 0b0_110_0100, 0b0_001_1111, 0b1_000_0001];
        let mut buffer: Vec<u8> = vec![];
        VarUInt::write_var_uint(&mut buffer, value).unwrap();
        assert_eq!(buffer.len(), expected_bytes.len());
        assert_eq!(buffer, expected_bytes);
    }

    #[test]
    fn test_write_var_uint_2() {
        let value: u64 = 0;
        let expected_bytes = &[0b1_000_0000];
        let mut buffer: Vec<u8> = vec![];
        VarUInt::write_var_uint(&mut buffer, value).unwrap();
        assert_eq!(buffer.len(), expected_bytes.len());
        assert_eq!(buffer, expected_bytes);
    }

    #[test]
    fn test_write_var_uint_3() {
        let value: u64 = 1;
        let expected_bytes = &[0b1_000_0001];
        let mut buffer: Vec<u8> = vec![];
        VarUInt::write_var_uint(&mut buffer, value).unwrap();
        assert_eq!(buffer.len(), expected_bytes.len());
        assert_eq!(buffer, expected_bytes);
    }
}
