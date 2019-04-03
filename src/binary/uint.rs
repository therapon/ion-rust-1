use crate::result::IonResult;
use std::io::Read;
use std::io::Write;
use std::mem;

const SIZE_OF_U64: u32 = mem::size_of::<u64>() as u32;
type UIntStorage = u64;

#[derive(Debug)]
pub struct UInt {
    size_in_bytes: usize,
    value: UIntStorage,
}

impl UInt {
    #[inline]
    pub fn read_uint<R: Read>(data_source: &mut R, length: usize) -> IonResult<UInt> {
        let mut magnitude: UIntStorage = 0;
        let mut buffer = [0u8; 8]; //TODO: Pass in a buffer instead of specifying a length
        let buffer = &mut buffer[0..length];
        //TODO: Read `length` bytes at once instead of error handling each byte
        data_source.read_exact(buffer)?;
        for byte in buffer.iter().cloned() {
            let byte = u64::from(byte);
            magnitude <<= 8;
            magnitude |= byte;
        }
        Ok(UInt {
            size_in_bytes: length,
            value: magnitude,
        })
    }

    #[inline]
    pub fn write_uint<W: Write>(sink: &mut W, magnitude: u64) -> IonResult<()> {
        // leading_zeros() uses an intrinsic to calculate this quickly
        let empty_leading_bytes: u32 = magnitude.leading_zeros() >> 3; // Divide by 8 to get byte count
        let first_occupied_byte = empty_leading_bytes as usize;

        let magnitude_bytes: [u8; mem::size_of::<u64>()] = magnitude.to_be_bytes();
        let bytes_to_write: &[u8] = &magnitude_bytes[first_occupied_byte..];

        println!("Writing as {:?}", bytes_to_write);
        sink.write_all(bytes_to_write)?;
        Ok(())
    }

    #[inline(always)]
    pub fn value(&self) -> UIntStorage {
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
    fn test_write_uint() {
        let value = 0x01_23_45_67_89_AB_CD_EF;
        let mut buffer: Vec<u8> = vec![];
        UInt::write_uint(&mut buffer, value).expect("Write failed.");
        let expected_bytes = &[0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
        assert_eq!(expected_bytes, buffer.as_slice());

        let value = 0x01_23_45_67_89;
        let mut buffer: Vec<u8> = vec![];
        UInt::write_uint(&mut buffer, value).expect("Write failed.");
        let expected_bytes = &[0x01, 0x23, 0x45, 0x67, 0x89];
        assert_eq!(expected_bytes, buffer.as_slice());

        let value = 0x01_23_45;
        let mut buffer: Vec<u8> = vec![];
        UInt::write_uint(&mut buffer, value).expect("Write failed.");
        let expected_bytes: &[u8] = &[0x01, 0x23, 0x45];
        assert_eq!(expected_bytes, buffer.as_slice());

        let value = 0x00;
        let mut buffer: Vec<u8> = vec![];
        UInt::write_uint(&mut buffer, value).expect("Write failed.");
        let expected_bytes: &[u8] = &[];
        assert_eq!(expected_bytes, buffer.as_slice());
    }

    #[test]
    fn test_read_uint() {
        let error_message = "Failed to read a UInt from the provided cursor.";
        let data = &[0b0011_1100, 0b1000_0111, 0b1000_0001];
        let uint = UInt::read_uint(&mut Cursor::new(data), data.len()).expect(error_message);
        assert_eq!(uint.size_in_bytes(), 3);
        assert_eq!(uint.value(), 3_966_849);

        let data = &[0b1000_0000];
        let uint = UInt::read_uint(&mut Cursor::new(data), data.len()).expect(error_message);
        assert_eq!(uint.size_in_bytes(), 1);
        assert_eq!(uint.value(), 128);

        let data = &[0b0111_1111, 0b1111_1111];
        let uint = UInt::read_uint(&mut Cursor::new(data), data.len()).expect(error_message);
        assert_eq!(uint.size_in_bytes(), 2);
        assert_eq!(uint.value(), 32_767);
    }
}
