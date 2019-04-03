use std::io::Read;

use crate::result::IonResult;

// TODO: variable size storage
type VarIntStorage = isize;

#[derive(Debug)]
pub struct VarInt {
    size_in_bytes: usize,
    value: VarIntStorage,
}

impl VarInt {
    #[inline(always)]
    pub fn read_var_int(data_source: &mut impl Read) -> IonResult<VarInt> {
        // Unlike VarUInt (note the U), the first byte in VarInt is a special case because
        // bit #6 (0-indexed, from the right) indicates whether the value is positive (0) or
        // negative (1).

        let first_byte: u8 = data_source.bytes().next().unwrap()?;
        let is_positive: bool = (first_byte & 0b0100_0000) == 0;
        let sign: VarIntStorage = if is_positive { 1 } else { -1 };

        let mut number_of_bytes = 1;
        let mut magnitude = (first_byte & 0b0011_1111) as VarIntStorage;

        let no_more_bytes: bool = first_byte >= 0b1000_0000; // If the first bit is 1, we're done.

        if no_more_bytes {
            return Ok(VarInt {
                size_in_bytes: number_of_bytes,
                value: magnitude * sign,
            });
        }

        // All of the other bytes are handled in a manner similar to VarUInt.
        // TODO: Optimize this by using the BufRead interface as we've done in VarUInt
        for byte_result in data_source.bytes() {
            let byte = byte_result?;
            number_of_bytes += 1;
            let lower_seven = 0b0111_1111 & byte;
            let lower_seven = lower_seven as VarIntStorage;
            magnitude <<= 7;
            magnitude |= lower_seven;
            if byte >= 0b1000_0000 {
                break;
            }
        }
        Ok(VarInt {
            size_in_bytes: number_of_bytes,
            value: magnitude * sign,
        })
    }

    #[inline(always)]
    pub fn value(&self) -> VarIntStorage {
        self.value
    }

    #[inline(always)]
    pub fn size_in_bytes(&self) -> usize {
        self.size_in_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::VarInt;
    use std::io::Cursor;

    #[test]
    fn test_read_var_int() {
        let error_message = "Failed to read a Varint from the provided cursor.";
        let varint =
            VarInt::read_var_int(&mut Cursor::new(&[0b0111_1001, 0b0000_1111, 0b1000_0001]))
                .expect(error_message);
        assert_eq!(varint.size_in_bytes(), 3);
        assert_eq!(varint.value(), -935_809);
        let varint =
            VarInt::read_var_int(&mut Cursor::new(&[0b0011_1001, 0b0000_1111, 0b1000_0001]))
                .expect(error_message);
        assert_eq!(varint.size_in_bytes(), 3);
        assert_eq!(varint.value(), 935_809);
        let varint = VarInt::read_var_int(&mut Cursor::new(&[0b1000_0000])).expect(error_message);
        assert_eq!(varint.size_in_bytes(), 1);
        assert_eq!(varint.value(), 0);
        let varint = VarInt::read_var_int(&mut Cursor::new(&[0b0111_1111, 0b1111_1111]))
            .expect(error_message);
        assert_eq!(varint.size_in_bytes(), 2);
        assert_eq!(varint.value(), -8_191);
        let varint = VarInt::read_var_int(&mut Cursor::new(&[0b0011_1111, 0b1111_1111]))
            .expect(error_message);
        assert_eq!(varint.size_in_bytes(), 2);
        assert_eq!(varint.value(), 8_191);
    }
}
