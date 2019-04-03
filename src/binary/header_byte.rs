use crate::binary::ion_cursor::IonValueHeader;
use crate::binary::ion_type_code::IonTypeCode;
use crate::result::IonResult;

const MAX_NIBBLE_VALUE: u8 = 15;

/* TODO
   Each binary Ion value has (at minimum) a one-byte header that indicates its type, the number
   of bytes used to represent the value, and occasionally the value itself. Because it is only a
   single byte, we can pre-calculate all 256 possible values and cache them in an array.

   However, until const functions are more sophisticated, there are only two ways to share this
   immutable array across all threads and readers:

    1. Write out the complete definition of the array by hand and store it as a public const value.
    2. Use the lazy_static! macro to lazily calculate the array when it's first read and then allow
       interested readers to reference that. Because lazy_static! uses a smartpointer, dereferencing
       its values involves a fair amount of overhead and is bad for performance sensitive
       applications. However, you can skirt much of the issue by simply having each IonSystem
       clone the array at initialization time. This approach is currently being used to
       avoid unnecessary complexity.

    Once const functions can handle logic like match statements,
    (https://github.com/rust-lang/rust/issues/57563) we can migrate to that.
*/
lazy_static! {
    pub(crate) static ref SLOW_HEADERS: Vec<IonResult<Option<IonValueHeader>>> = {
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
        length_code,
    }))
}

pub fn nibbles_from_byte(byte: u8) -> (u8, u8) {
    let left = byte >> 4;
    let right = byte & 0b1111;
    (left, right)
}

pub fn byte_from_nibbles(left: u8, right: u8) -> u8 {
    assert!(left <= MAX_NIBBLE_VALUE);
    assert!(right <= MAX_NIBBLE_VALUE);
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
