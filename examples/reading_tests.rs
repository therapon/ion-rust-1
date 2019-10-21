extern crate amzn_ion;

use std::fs::File;
use std::io::BufReader;
use std::io;
use amzn_ion::result::IonResult;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;

fn reading_tests() -> IonResult<()> {
  let data: &[u8] = &[
    // IVM
    0xE0, 0x01, 0x00, 0xEA,
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1100_0011, // -3 as VarInt
    // Coefficient
    0b0000_0111 // 7 as Int
  ];
  let mut input = io::Cursor::new(data);
  let mut cursor = BinaryIonCursor::new(&mut input)?;
  let ion_type = cursor.next()?.unwrap();
  let decimal = cursor
    .read_decimal()?
    .unwrap();

  println!("Decimal value: {:?}", decimal);

  let data: &[u8] = &[
    // IVM
    0xE0, 0x01, 0x00, 0xEA,
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1000_0100, // 4 as VarInt
    // Coefficient
    0b0000_0101 // 5 as Int
  ];
  let mut input = io::Cursor::new(data);
  let mut cursor = BinaryIonCursor::new(&mut input)?;
  let ion_type = cursor.next()?.unwrap();
  let decimal = cursor
    .read_decimal()?
    .unwrap();

  println!("Decimal value: {:?}", decimal);

  let data: &[u8] = &[
    // IVM
    0xE0, 0x01, 0x00, 0xEA,
    // Decimal, 2 bytes
    0x52,
    // Exponent
    0b1100_0011, // -3 as VarInt
    // Coefficient
    0b0000_0111 // 7 as Int
  ];
  let mut input = io::Cursor::new(data);
  let mut cursor = BinaryIonCursor::new(&mut input)?;
  let ion_type = cursor.next()?.unwrap();
  let decimal = cursor
    .read_decimal()?
    .unwrap();

  println!("Decimal value: {:?}", decimal);
  Ok(())
}

fn main() {
  match reading_tests() {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}