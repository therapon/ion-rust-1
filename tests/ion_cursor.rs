extern crate amzn_ion;
extern crate bigdecimal;
extern crate num_bigint;

use std::io;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;
use amzn_ion::types::ion_type::IonType;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use amzn_ion::types::ion_decimal::IonDecimal;

fn ivm() -> Vec<u8> {
  let mut ivm = Vec::new();
  ivm.extend(&[0xE0, 0x01, 0x00, 0xEA]);
  ivm
}

fn ion_data(bytes: &[u8]) -> Vec<u8> {
  let mut data = ivm();
  data.extend(bytes.into_iter());
  data
}

fn io_cursor_for(bytes: &[u8]) -> io::Cursor<Vec<u8>> {
  let data = ion_data(bytes);
  io::Cursor::new(data)
}

fn advance_to(cursor: &mut BinaryIonCursor<io::Cursor<Vec<u8>>>, expected_ion_type: IonType) {
  let ion_type = cursor.next().unwrap().unwrap();
  assert_eq!(ion_type, expected_ion_type);
}

fn decimal_reading_test(bytes: &[u8], expected: IonDecimal) {
  let mut io_cursor = io_cursor_for(bytes);
  let mut cursor = BinaryIonCursor::new(&mut io_cursor).unwrap();
  advance_to(&mut cursor, IonType::Decimal);
  let ion_decimal = cursor
    .decimal_value()
    .unwrap()
    .unwrap();

  assert_eq!(ion_decimal, expected);
}

#[test]
fn read_positive_decimal_positive_exponent() {
  let data = &[
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1000_0011, // 3 as VarInt
    // Coefficient
    0b0000_0111 // 7 as Int
  ];
  let expected: IonDecimal = BigDecimal::new(7i64.into(), 3i64 * -1).into();
  decimal_reading_test(data, expected);
}

#[test]
fn read_negative_decimal_positive_exponent() {
  let data = &[
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1000_0011, // 3 as VarInt
    // Coefficient
    0b1000_0111 // -7 as Int
  ];
  let expected: IonDecimal = BigDecimal::new((-7i64).into(), 3i64 * -1).into();
  decimal_reading_test(data, expected);
}

#[test]
fn read_negative_decimal_negative_exponent() {
  let data = &[
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1100_0011, // -3 as VarInt
    // Coefficient
    0b1000_0111 // -7 as Int
  ];
  let expected: IonDecimal = BigDecimal::new((-7i64).into(), -3i64 * -1).into();
  decimal_reading_test(data, expected);
}

#[test]
fn read_positive_decimal_negative_exponent() {
  let data = &[
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1100_0011, // -3 as VarInt
    // Coefficient
    0b0000_0111 // 7 as Int
  ];
  let expected: IonDecimal = BigDecimal::new((7i64).into(), -3i64 * -1).into();
  decimal_reading_test(data, expected);
}