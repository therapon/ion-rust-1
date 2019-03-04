use std::io::Write;
use std::mem;

use bytes::BigEndian;
use bytes::ByteOrder;

use binary::uint::UInt;
use result::IonResult;
use types::IonType;
use binary::var_uint::VarUInt;

pub struct BinaryIonWriter<W: Write> {
  sink: W
}

impl<W: Write> BinaryIonWriter<W> {
  pub fn new(sink: W) -> Self {
    BinaryIonWriter {
      sink
    }
  }

  pub fn write_null(&mut self, ion_type: IonType) -> IonResult<()> {
    use types::IonType::*;
    let null_byte: u8 = match ion_type {
      Null => 0b0000_1111,
      Boolean => 0b0001_1111,
      Integer => 0b0010_1111, // Only ever writes the 'Positive Integer' version of Null
      Float => 0b0100_1111,
      Decimal => 0b0101_1111,
      Timestamp => 0b0110_1111,
      Symbol => 0b0111_1111,
      String => 0b1000_1111,
      Clob => 0b1001_1111,
      Blob => 0b1010_1111,
      List => 0b1011_1111,
      SExpression => 0b1100_1111,
      Struct => 0b1101_1111
    };
    self.sink.write_all(&[null_byte])?;
    Ok(())
  }

  pub fn write_bool(&mut self, value: bool) -> IonResult<()> {
    let boolean_byte = match value {
      true => 0b0001_0001,
      false => 0b0001_0000
    };
    self.sink.write_all(&[boolean_byte])?;
    Ok(())
  }

  pub fn write_i64(&mut self, value: i64) -> IonResult<()> {
    let magnitude: u64 = value.abs() as u64; // Lossless conversion
    println!("magnitude: {}", magnitude);
    // If the value fits inside of an i64, its encoded 'L' value must be 8 or lower.
    let size_in_bytes = Self::uint_size_in_bytes(magnitude) as u8;
    println!("Writing i64 value {}, size in bytes {}", value, size_in_bytes);
    let header_byte: u8 = if value >= 0 {
      // Positive Integer: 2
      0b0010_0000 | size_in_bytes
    } else {
      // Negative Integer: 3
      0b0011_0000 | size_in_bytes
    };
    self.sink.write_all(&[header_byte])?;
    UInt::write_uint(&mut self.sink, magnitude)?;
    Ok(())
  }

  pub fn write_f32(&mut self, value: f32) -> IonResult<()> {
    let mut encoding_buffer: [u8; mem::size_of::<f32>()] = [0; mem::size_of::<f32>()];
    if value == 0e0 {
      // Type code 4 (Float), L code 0 (zero value, no body required)
      self.sink.write_all(&[0b0100_0000])?;
    } else {
      // Type code 4 (Float), L code 4 (f32)
      self.sink.write_all(&[0b0100_0100])?;
      BigEndian::write_f32(&mut encoding_buffer, value);
      self.sink.write_all(&encoding_buffer)?;
    };

    Ok(())
  }

  pub fn write_f64(&mut self, value: f64) -> IonResult<()> {
    let mut encoding_buffer: [u8; mem::size_of::<f64>()] = [0; mem::size_of::<f64>()];
    if value == 0e0 {
      // Type code 4 (Float), L code 0 (zero value, no body required)
      self.sink.write_all(&[0b0100_0000])?;
    } else {
      // Type code 4 (Float), L code 8 (f64)
      self.sink.write_all(&[0b0100_1000])?;
      BigEndian::write_f64(&mut encoding_buffer, value);
      self.sink.write_all(&encoding_buffer)?;
    };
    Ok(())
  }

  pub fn write_string(&mut self, string: &str) -> IonResult<()> {
    let text: &str = string.as_ref();
    let size_in_bytes = text.as_bytes().len();
    // If the size is greater than or equal to 14, we'll need to encode it as a VarUInt.
    if size_in_bytes >= 14 { //TODO: Magic number.
      let header_byte: u8 = 0b1000_1110; // Type code 8 (String), length code 14 (VarUInt)
      self.sink.write_all(&[header_byte])?;
      VarUInt::write_var_uint(&mut self.sink, size_in_bytes as u64)?;
    } else {
      let header_byte: u8 = 0b1000_0000 | (size_in_bytes as u8);
      self.sink.write_all(&[header_byte])?;
    }
    self.sink.write_all(text.as_bytes())?;
    Ok(())
  }

  fn uint_size_in_bytes(value: u64) -> u32 {
    let empty_leading_bytes: u32 = value.leading_zeros() >> 3; // Divide by 8 to get byte count
    mem::size_of::<u64>() as u32 - empty_leading_bytes
  }
}

#[cfg(test)]
mod tests {
  use super::BinaryIonWriter;
  use binary::BinaryIonReader;
  use std::io;
  use types::IonType;
  use result::IonResult;
  use std::io::Write;
  use binary::ion_cursor::IonDataSource;
  use std::fmt::Debug;

  fn create_ion_buffer() -> Vec<u8> {
    vec![0xE0u8, 0x01, 0x00, 0xEA]
  }

  //TODO: Add these to the Reader? These seem generally useful for validation.

  fn expect<R: IonDataSource, F: Debug + PartialEq>(reader: &mut BinaryIonReader<R>,
                                                    ion_type: IonType,
                                                    value_reader: fn(&mut BinaryIonReader<R>) -> IonResult<Option<F>>,
                                                    value: F) {
    assert_eq!(ion_type, reader.next().unwrap().unwrap());
    assert_eq!(value, value_reader(reader).unwrap().unwrap());
  }

  fn expect_i64<R: IonDataSource>(reader: &mut BinaryIonReader<R>,
                                  value: i64) {
    expect(reader, IonType::Integer, BinaryIonReader::read_i64, value);
  }

  fn expect_f32<R: IonDataSource>(reader: &mut BinaryIonReader<R>, value: f32) {
    expect(reader, IonType::Float, BinaryIonReader::read_f32, value);
  }

  fn expect_f64<R: IonDataSource>(reader: &mut BinaryIonReader<R>, value: f64) {
    expect(reader, IonType::Float, BinaryIonReader::read_f64, value);
  }

  fn expect_string<R: IonDataSource>(reader: &mut BinaryIonReader<R>, value: &str) {
    expect(reader, IonType::String, BinaryIonReader::read_string, value.to_owned());
  }

  fn expect_typed_null<R: IonDataSource>(reader: &mut BinaryIonReader<R>, ion_type: IonType) {
    assert_eq!(ion_type, reader.next().unwrap().unwrap());
    assert_eq!(true, reader.is_null());
  }

  #[test]
  fn test_round_trip_null() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_null(IonType::Null).unwrap();
    writer.write_null(IonType::Boolean).unwrap();
    writer.write_null(IonType::Integer).unwrap();
    writer.write_null(IonType::Float).unwrap();
    writer.write_null(IonType::Decimal).unwrap();
    writer.write_null(IonType::Timestamp).unwrap();
    writer.write_null(IonType::Symbol).unwrap();
    writer.write_null(IonType::String).unwrap();
    writer.write_null(IonType::Clob).unwrap();
    writer.write_null(IonType::Blob).unwrap();
    writer.write_null(IonType::List).unwrap();
    writer.write_null(IonType::Struct).unwrap();
    writer.write_null(IonType::SExpression).unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_typed_null(&mut reader, IonType::Null);
    expect_typed_null(&mut reader, IonType::Boolean);
    expect_typed_null(&mut reader, IonType::Integer);
    expect_typed_null(&mut reader, IonType::Float);
    expect_typed_null(&mut reader, IonType::Decimal);
    expect_typed_null(&mut reader, IonType::Timestamp);
    expect_typed_null(&mut reader, IonType::Symbol);
    expect_typed_null(&mut reader, IonType::String);
    expect_typed_null(&mut reader, IonType::Clob);
    expect_typed_null(&mut reader, IonType::Blob);
    expect_typed_null(&mut reader, IonType::List);
    expect_typed_null(&mut reader, IonType::Struct);
    expect_typed_null(&mut reader, IonType::SExpression);
  }

  #[test]
  fn test_round_trip_i64() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_i64(-8_675_309).unwrap();
    writer.write_i64(-786).unwrap();
    writer.write_i64(-42).unwrap();
    writer.write_i64(0).unwrap();
    writer.write_i64(42).unwrap();
    writer.write_i64(786).unwrap();
    writer.write_i64(8_675_309).unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_i64(&mut reader, -8_675_309);
    expect_i64(&mut reader, -786);
    expect_i64(&mut reader, -42);
    expect_i64(&mut reader, 0);
    expect_i64(&mut reader, 42);
    expect_i64(&mut reader, 786);
    expect_i64(&mut reader, 8_675_309);
  }

  #[test]
  fn test_round_trip_f32() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_f32(0.0).unwrap();
    writer.write_f32(1.0).unwrap();
    writer.write_f32(-1.0).unwrap();
    writer.write_f32(3.14).unwrap();
    writer.write_f32(-3.14).unwrap();
    writer.write_f32(3.1415926535).unwrap();
    writer.write_f32(-3.1415926535).unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_f32(&mut reader, 0.0);
    expect_f32(&mut reader, 1.0);
    expect_f32(&mut reader, -1.0);
    expect_f32(&mut reader, 3.14);
    expect_f32(&mut reader, -3.14);
    expect_f32(&mut reader, 3.1415926535);
    expect_f32(&mut reader, -3.1415926535);
  }

  #[test]
  fn test_round_trip_f64() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_f64(0.0).unwrap();
    writer.write_f64(1.0).unwrap();
    writer.write_f64(-1.0).unwrap();
    writer.write_f64(3.14).unwrap();
    writer.write_f64(-3.14).unwrap();
    writer.write_f64(3.1415926535).unwrap();
    writer.write_f64(-3.1415926535).unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_f64(&mut reader, 0.0);
    expect_f64(&mut reader, 1.0);
    expect_f64(&mut reader, -1.0);
    expect_f64(&mut reader, 3.14);
    expect_f64(&mut reader, -3.14);
    expect_f64(&mut reader, 3.1415926535);
    expect_f64(&mut reader, -3.1415926535);
  }

  #[test]
  fn test_round_trip_string() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_string("So long.").unwrap();
    writer.write_string("Farewell").unwrap();
    writer.write_string("Auf Wiedersehen").unwrap();
    writer.write_string("Goodbye").unwrap();
    writer.write_string("Adieu").unwrap();
    writer.write_string("Toodles").unwrap();
    writer.write_string("Sayonara").unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_string(&mut reader, "So long.");
    expect_string(&mut reader, "Farewell");
    expect_string(&mut reader, "Auf Wiedersehen");
    expect_string(&mut reader, "Goodbye");
    expect_string(&mut reader, "Adieu");
    expect_string(&mut reader, "Toodles");
    expect_string(&mut reader, "Sayonara");
  }

  #[test]
  fn test_round_trip_mixed_types() {
    let mut buffer = create_ion_buffer();
    let mut writer = BinaryIonWriter::new(&mut buffer);

    writer.write_string("So long.").unwrap();
    writer.write_i64(-1_024).unwrap();
    writer.write_string("Farewell").unwrap();
    writer.write_i64(-256).unwrap();
    writer.write_i64(-0).unwrap();
    writer.write_string("Auf Wiedersehen").unwrap();
    writer.write_i64(0).unwrap();
    writer.write_i64(256).unwrap();
    writer.write_i64(1_024).unwrap();

    println!("buffer after writing: {:?}", buffer.as_slice());

    let io_cursor = io::Cursor::new(buffer.as_slice());
    let mut reader = BinaryIonReader::new(io_cursor)
      .expect("Couldn't create reader.");

    expect_string(&mut reader, "So long.");
    expect_i64(&mut reader, -1_024);
    expect_string(&mut reader, "Farewell");
    expect_i64(&mut reader, -256);
    expect_i64(&mut reader, -0);
    expect_string(&mut reader, "Auf Wiedersehen");
    expect_i64(&mut reader, 0);
    expect_i64(&mut reader, 256);
    expect_i64(&mut reader, 1_024);
  }
}