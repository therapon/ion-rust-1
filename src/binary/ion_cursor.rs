use std::error::Error;
use std::io::{Read, Seek};
use std::collections::vec_deque::VecDeque;

use bytes::BigEndian;
use bytes::ByteOrder;

use smallvec::SmallVec;

use binary::header_byte::*;
use binary::var_uint::VarUInt;
use binary::ion_type_code::IonTypeCode;
use binary::uint::UInt;

use types::*;

use result::{IonResult, IonError, io_error, decoding_error};

use std::ops::Deref;
use std::ops::DerefMut;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
//use types::ion_string::IonStringRef;
//use types::ion_boolean::IonBoolean;
//use types::ion_integer::IonInteger;
//use types::ion_float::IonFloat;
//use types::ion_symbol::{IonSymbol, IonSymbolId};
//use types::ion_blob::IonBlobRef;
//use types::ion_clob::IonClobRef;
//use types::ion_timestamp::IonTimestamp;
use binary::var_int::VarInt;

use chrono::prelude::*;
use chrono::offset::FixedOffset;
//use types::ion_decimal::IonDecimal;
use binary::int::Int;
use bigdecimal::BigDecimal;
use std::io::ErrorKind;
use std::env::Args;
//use types::ion_string::IonString;
//use types::ion_blob::IonBlob;
//use types::ion_clob::IonClob;
//use types::ion_value::IonValue;
//use types::ion_null::IonNull;
//use types::ion_null::IonNull;

const LENGTH_CODE_NULL: u8 = 15;
const LENGTH_CODE_VAR_UINT: u8 = 14;

const IVM_LENGTH: usize = 4;
const IVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

//TODO: Crate pub, not pub-pub. Also: move to own module, possibly in header_byte?
#[derive(Copy, Clone, Debug)]
pub struct IonValueHeader {
  // We precalculate all of the possible IonValueHeaders during cursor init.
  pub ion_type: Option<IonType>,
  pub ion_type_code: IonTypeCode,
  pub length_code: u8
}

#[derive(Clone, Debug)]
struct CursorValue {
  ion_type: IonType, // TODO: Eliminate in favor of ion_type in header?
  header: IonValueHeader,
  is_null: bool,
  index_at_depth: usize, // The number of values read so far at this level
  length_in_bytes: usize,
  last_byte: usize,
  field_id: Option<IonSymbolId>,
//  annotations: SmallVec<[usize; 2]>,
  annotations: Vec<IonSymbolId>,
//  parent: Option<Rc<CursorValue>>,
  parent_index: Option<usize>
}

impl Default for CursorValue {
  fn default() -> CursorValue {
    CursorValue {
      ion_type: IonType::Null,
      header: IonValueHeader {
        ion_type: None,
        ion_type_code: IonTypeCode::Null,
        length_code: LENGTH_CODE_NULL,
      },
      field_id: None,
      annotations: Vec::new(),
      is_null: true,
      index_at_depth: 0,
      length_in_bytes: 0,
      last_byte: 0,
      parent_index: None,
    }
  }
}

/* CursorState is broken out from the BinaryIonCursor struct to allow it to be cloned
 * or replaced as part of a seek operation.
 */
#[derive(Clone, Debug)]
pub struct CursorState {
  bytes_read: usize, // How many bytes we've read from `data_source`
  depth: usize, // How deeply nested the cursor is at the moment
  index_at_depth: usize, // The number of values (starting with 0) read at the current depth
  is_in_struct: bool, // Whether this level of descent is within a struct
  value: CursorValue, // Information about the value on which the cursor is currently sitting.
}

// A low-level reader that offers no validation or symbol management.
// It can only move and return the current value.
pub struct BinaryIonCursor<R> where R: IonDataSource {
  data_source: BufReader<R>,
  buffer: Vec<u8>, // Used for individual data_source.read() calls independent of input buffering
  cursor: CursorState,
  parents: Vec<CursorValue>,
  // TODO: This should be a const living somewhere. Unfortunately, `lazy_static` adds a LOT of accessor overhead.
  header_cache: Vec<IonResult<Option<IonValueHeader>>>
}

pub trait IonDataSource: Read {
  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()>;
}

//default impl <T> DataSource for T where T: Read {
impl <T> IonDataSource for T where T: Read {
  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
    use std::io;
    let _bytes_copied = io::copy(
      &mut self.by_ref().take(number_of_bytes as u64),
      &mut io::sink()
    )?;
    Ok(())
  }
}

//impl <T> DataSource for BufReader<T> where T: Read + Seek {
//  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//    Ok(self.seek_relative(number_of_bytes as i64)?)
//  }
//}

//impl <T> DataSource for T where T: Read + Seek {
//  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//    use std::io::{Seek, SeekFrom};
//    let new_pos = (self as &mut Seek).seek(SeekFrom::Current(number_of_bytes as i64))?;
//    Ok(())
//  }
//}

impl <R> BinaryIonCursor<R> where R: IonDataSource + Seek {
  pub fn checkpoint(&self) -> CursorState {
    self.cursor.clone()
  }

  pub fn restore(&mut self, mut saved_state: CursorState) -> IonResult<()> {
   use std::mem;
    use std::io::{Seek, SeekFrom};
    mem::swap(&mut self.cursor, &mut saved_state);
    (self.data_source.get_mut() as &mut Seek).seek(SeekFrom::Start(self.cursor.bytes_read as u64))?;
    Ok(())
  }
}

//impl <'cursor> BinaryIonCursor<'cursor, BufReader<File>> {
//  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//    use std::io::{Seek, SeekFrom};
//    //debug!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);
//
//    (self.data_source as &mut Seek).seek(SeekFrom::Current(number_of_bytes as i64))?;
//
//    self.cursor.bytes_read += number_of_bytes;
//    //debug!("After seek, bytes_read: {}", self.cursor.bytes_read);
//    Ok(())
//  }
//}

impl <R> BinaryIonCursor<R> where R: IonDataSource {

  pub fn is_null(&self) -> bool {
    self.cursor.value.is_null
  }

  pub fn ion_type(&self) -> IonType {
    self.cursor.value.ion_type
  }

  pub fn depth(&self) -> usize {
    self.cursor.depth
  }

  pub fn integer_value(&mut self) -> IonResult<Option<IonInteger>> {
    use self::IonTypeCode::*;
    if self.cursor.value.ion_type != IonType::Integer ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same integer value more than once.");
    }

    let magnitude = self.read_value_as_uint()?.value();

    let value = match self.cursor.value.header.ion_type_code {
      PositiveInteger => IonInteger::from(magnitude as i64),
      NegativeInteger => IonInteger::from(magnitude as i64 * -1),
      _ => unreachable!("The Ion Type Code must be one of the above to reach this point.")
    };

    return Ok(Some(value));
  }

  pub fn float_value(&mut self) -> IonResult<Option<IonFloat>> {
    if self.cursor.value.ion_type != IonType::Float ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same float value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;

    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      let value = match number_of_bytes {
        0 => 0f64,
        4 => BigEndian::read_f32(buffer) as f64,
        8 => BigEndian::read_f64(buffer),
        _ => return decoding_error(
          &format!(
            "Encountered an illegal value for a Float length: {}",
            number_of_bytes
          )
        )
      };
      Ok(Some(IonFloat::from(value)))
    })
  }

  pub fn boolean_value(&mut self) -> Result<Option<IonBoolean>, IonError> {
    if self.cursor.value.ion_type != IonType::Boolean ||  self.cursor.value.is_null {
      return Ok(None);
    }

    // No reading from the stream occurs -- the header contained all of the information we needed.

    let representation = self.cursor.value.header.length_code;

    match representation {
      0 => Ok(Some(IonBoolean::from(false))),
      1 => Ok(Some(IonBoolean::from(true))),
      _ => decoding_error(
        &format!("Found a boolean value with an illegal representation: {}", representation)
      )
    }
  }

  pub fn string_value(&mut self) -> IonResult<Option<IonString>> {
    self.string_ref_map(|s: IonStringRef| s.to_string().into())
  }

  pub fn string_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonStringRef) -> T {
    use std::str;
    if self.cursor.value.ion_type != IonType::String || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value(){
      panic!("You cannot read the same string value more than once.");
    }

    let length_in_bytes = self.cursor.value.length_in_bytes;

    self.parse_n_bytes(length_in_bytes, |buffer: &[u8]| {
      let string_ref = match str::from_utf8(buffer) {
        Ok(utf8_text) => IonStringRef::from(utf8_text),
        Err(utf8_error) =>  return decoding_error(
          &format!(
            "The requested string was not valid UTF-8: {:?}",
            utf8_error
          )
        )
      };
      Ok(Some(f(string_ref)))
    })
  }

  pub fn symbol_id_value(&mut self) -> IonResult<Option<IonSymbolId>> {
    if self.cursor.value.ion_type != IonType::Symbol ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same symbol ID value more than once.");
    }

    let symbol_id = self.read_value_as_uint()?.value() as usize;
    Ok(Some(Into::into(symbol_id)))
  }

  pub fn blob_value(&mut self) -> IonResult<Option<IonBlob>> {
    self.blob_ref_map(|b| b.into())
  }

  pub fn blob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonBlobRef) -> T {
    if self.cursor.value.ion_type != IonType::Blob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same blob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      let blob_ref: IonBlobRef = From::from(buffer);
      Ok(Some(f(blob_ref)))
    })
  }

  pub fn clob_value(&mut self) -> IonResult<Option<IonClob>> {
    self.clob_ref_map(|c| c.into())
  }

  pub fn clob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonClobRef) -> T {
    if self.cursor.value.ion_type != IonType::Clob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same clob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      let clob_ref: IonClobRef = From::from(buffer);
      Ok(Some(f(clob_ref)))
    })
  }

  pub fn decimal_value(&mut self) -> IonResult<Option<IonDecimal>> {
    if self.cursor.value.ion_type != IonType::Decimal ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same decimal value more than once.");
    }

    let exponent_var_int = self.read_var_int()?;
    let coefficient_size_in_bytes = self.cursor.value.length_in_bytes - exponent_var_int.size_in_bytes();

    let exponent = exponent_var_int.value() as i64;
    let coefficient = self.read_int(coefficient_size_in_bytes)?.value();

    // BigDecimal uses 'scale' rather than 'exponent' in its API, which is a count of the number of
    // decimal places. It's effectively `exponent * -1`.
    Ok(Some(BigDecimal::new(coefficient.into(), exponent * -1).into()))
  }

  fn finished_reading_value(&mut self) -> bool {
    self.cursor.bytes_read >= self.cursor.value.last_byte
  }

  pub fn timestamp_value(&mut self) -> IonResult<Option<IonTimestamp>> {
    if self.cursor.value.ion_type != IonType::Timestamp ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same timestamp value more than once.");
    }

    let offset_minutes = self.read_var_int()?.value();
    let year = self.read_var_uint()?.value();

    let mut month: usize = 0;
    let mut day: usize = 0;
    let mut hour: usize = 0;
    let mut minute: usize = 0;
    let mut second: usize = 0;
    // TODO:  Fractional seconds
//    let mut fraction_exponent = 0;
//    let mut fraction_coefficient = 0;

    loop {
      if self.finished_reading_value() {
        break;
      }
      month = self.read_var_uint()?.value();
      if self.finished_reading_value() {
        break;
      }

      day = self.read_var_uint()?.value();
      if self.finished_reading_value() {
        break;
      }

      hour = self.read_var_uint()?.value();
      if self.finished_reading_value() {
        break;
      }

      minute = self.read_var_uint()?.value();
      if self.finished_reading_value() {
        break;
      }

      second = self.read_var_uint()?.value();
      break;

      // TODO: Fractional seconds
    }

    let naive_datetime =
      NaiveDate::from_ymd(year as i32, month as u32, day as u32)
                .and_hms(hour as u32, minute as u32, second as u32);
    let offset = FixedOffset::west(offset_minutes as i32 * 60i32);
    let datetime = offset.from_utc_datetime(&naive_datetime);
    Ok(Some(From::from(datetime)))
  }

  pub fn annotation_ids<'a>(&'a self) -> impl Iterator<Item=IonSymbolId> + 'a {
    self.cursor.value.annotations.iter().cloned()
  }

  pub fn field_id(&self) -> Option<IonSymbolId> {
    self.cursor.value.field_id
  }

  pub fn value_is_symbol_table(&self) -> bool {
    let symbol_id = match (self.ion_type(), self.annotation_ids().next()) {
      (IonType::Struct, Some(symbol_id)) => symbol_id,
      _ => return false
    };
    return symbol_id == IonSymbolId::from(3u64)
  }

  pub fn new(mut data_source: R) -> IonResult<BinaryIonCursor<R>> {
    let buffer = &mut [0u8; 4];
    let _ = data_source.read_exact(buffer)?;
    if *buffer != IVM {
      return decoding_error(
        &format!(
          "The data source must begin with an Ion Version Marker ({:?}). Found: ({:?})",
          IVM,
          buffer
        )
      );
    }

    Ok(BinaryIonCursor {
      data_source: BufReader::with_capacity(32 * 1024, data_source),
      buffer: vec![0; 1024],
      cursor: CursorState {
        bytes_read: IVM_LENGTH,
        depth: 0,
        index_at_depth: 0,
        is_in_struct: false,
        value: Default::default()
      },
      parents: Vec::new(),
      header_cache: SLOW_HEADERS.clone(), // TODO: Bad. Make this static.
    })
  }

  pub fn next(&mut self) -> IonResult<Option<IonType>> {
    let _ = self.skip_current_value()?;

    if let Some(ref parent) = self.parents.last() {
      // If the cursor is nested inside a parent object, don't attempt to read beyond the end of
      // the parent. Users can call '.step_out()' to progress beyond the container.
      if self.cursor.bytes_read >= parent.last_byte {
        //debug!("We've run out of values in this parent.");
        return Ok(None);
      }
    }

    // If we're in a struct, read the field id that must precede each value.
    self.cursor.value.field_id = match self.cursor.is_in_struct {
      true => Some(self.read_field_id()?),
      false => None
    };

    // Pull the next byte from the data source and interpret it as a value header
    let mut header = match self.read_next_value_header()? {
      Some(header) => header,
      None => return Ok(None) // TODO: update ion_type() value to be None?
    };
    self.cursor.value.header = header;

    // Clear the annotations vec before (maybe) reading new ones
    self.cursor.value.annotations.truncate(0);
    if header.ion_type_code == IonTypeCode::Annotation {
      // We've found an annotated value. Read all of the annotation symbols leading up to the value.
      let _ = self.read_annotations()?;
      // Now read the next header representing the value itself.
      header = match self.read_next_value_header()? {
        Some(header) => header,
        None => return Ok(None)
      };
      self.cursor.value.header = header;
    }

    let _ = self.process_header_by_type_code(&header)?;

    self.cursor.index_at_depth += 1;
    self.cursor.value.index_at_depth = self.cursor.index_at_depth;

    Ok(Some(self.cursor.value.ion_type))
  }

  fn read_var_uint(&mut self) -> IonResult<VarUInt> {
    let var_uint = VarUInt::read_var_uint(&mut self.data_source)?;
    self.cursor.bytes_read += var_uint.size_in_bytes();
    Ok(var_uint)
  }

  fn read_var_int(&mut self) -> IonResult<VarInt> {
    let var_int = VarInt::read_var_int(&mut self.data_source)?;
    self.cursor.bytes_read += var_int.size_in_bytes() as usize;
    Ok(var_int)
  }

  // Useful when the entire value (all bytes after the type/length header) constitute a single UInt
  fn read_value_as_uint(&mut self) -> IonResult<UInt> {
    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.read_uint(number_of_bytes)
  }

  fn read_uint(&mut self, number_of_bytes: usize) -> IonResult<UInt> {
    let uint = UInt::read_uint(&mut self.data_source, number_of_bytes)?;
    self.cursor.bytes_read += uint.size_in_bytes();
    Ok(uint)
  }

  fn read_int(&mut self, number_of_bytes: usize) -> IonResult<Int> {
    let int = Int::read_int(&mut self.data_source, number_of_bytes)?;
    self.cursor.bytes_read += int.size_in_bytes();
    Ok(int)
  }

  fn read_exact(&mut self, number_of_bytes: usize) -> IonResult<()> {
    // It would be nice to use the buffer Vec as-is when it has enough capacity. Unfortunately,
    // Vec doesn't allow you to get a reference to its underlying [T] array beyond .len().
    // Thus, we can only optimize this for the case where the vec's .len() is larger than what we need.
    // As a minor hack, we can ensure the vec is always full of 0s so that len() == capacity.

    let buffer: &mut [u8];
    // Grow the Vec if needed
    if self.buffer.len() < number_of_bytes {
      self.buffer.resize(number_of_bytes, 0);
      buffer = self.buffer.as_mut();
    } else {
      // Otherwise, split the Vec's underlying storage to get a &mut [u8] slice of the required size
      let (required_buffer, _) = self.buffer.split_at_mut(number_of_bytes);
      buffer = required_buffer;
    }

    let _ = self.data_source.read_exact(buffer)?;
    self.cursor.bytes_read += number_of_bytes;
    Ok(())
  }


  // TODO: This function doesn't work when T is a reference because we can't consume the bytes if
  // the return value is still holding onto them.
  fn parse_n_bytes<T, F>(&mut self, number_of_bytes: usize, mut processor: F) -> IonResult<T>
    where F: FnMut(&[u8]) -> IonResult<T> {

    // If the requested value is already in our BufReader, there's no need to copy it out into a
    // separate buffer. We can return a slice of the BufReader buffer and consume() that number of
    // bytes.

    if self.data_source.buffer().len() >= number_of_bytes {
//      println!("We have {} bytes, we need {} bytes.", self.data_source.buffer().len(), number_of_bytes);
      let result = processor(&self.data_source.buffer()[..number_of_bytes]);
      self.data_source.consume(number_of_bytes);
      self.cursor.bytes_read += number_of_bytes;
      return result;
    }

    // TODO: Is it worth using .fill_buf() if number_of_bytes < self.data_source.capacity()?
    // It's likely better, but would only optimize reads on the cusp of the buffer.

    // Otherwise, read the value into self.buffer, a growable Vec.
    // It would be nice to use the buffer Vec as-is when it has enough capacity. Unfortunately,
    // Vec doesn't allow you to get a reference to its underlying [T] array beyond .len().
    // Thus, we can only optimize this for the case where the vec's .len() is larger than what we need.
    // As a minor hack, we can ensure the vec is always full of 0s so that len() == capacity.

    let buffer: &mut [u8];
    // Grow the Vec if needed
    if self.buffer.len() < number_of_bytes {
      self.buffer.resize(number_of_bytes, 0);
      buffer = self.buffer.as_mut();
    } else {
      // Otherwise, split the Vec's underlying storage to get a &mut [u8] slice of the required size
      let (required_buffer, _) = self.buffer.split_at_mut(number_of_bytes);
      buffer = required_buffer;
    }

    let _ = self.data_source.read_exact(buffer)?;
    let result = processor(buffer);
    self.cursor.bytes_read += number_of_bytes;
    result
  }

  fn process_header_by_type_code(&mut self, header: &IonValueHeader) -> IonResult<()> {
    use self::IonTypeCode::*;

//    self.cursor.value.ion_type = header.ion_type_code.as_type()?;
    self.cursor.value.ion_type = header.ion_type.unwrap();
    self.cursor.value.header = *header;
    self.cursor.value.is_null = header.length_code == LENGTH_CODE_NULL;

    let length = match header.ion_type_code {
      Null
      | Boolean => 0,
      PositiveInteger
      | NegativeInteger
      | Decimal
      | Timestamp
      | String
      | Symbol
      | List
      | SExpression
      | Clob
      | Blob => self.read_standard_length()?,
      Float => self.read_float_length()?,
      Struct => self.read_struct_length()?,
      Annotation => return decoding_error("Found an annotation wrapping an annotation."),
      Reserved => return decoding_error("Found an Ion Value with a Reserved type code.")
    };

    //debug!("Inferred length in bytes: {}", length);
    self.cursor.value.length_in_bytes = length;
    self.cursor.value.last_byte = self.cursor.bytes_read + length;
    //debug!("Cursor has read {} bytes. Last byte for this {:?}: {}",
//             self.cursor.bytes_read,
//             self.cursor.value.ion_type,
//             self.cursor.value.last_byte
//    );
    Ok(())
  }

  fn read_standard_length(&mut self) -> IonResult<usize> {
    let length = match self.cursor.value.header.length_code {
      LENGTH_CODE_NULL => 0,
      LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_float_length(&mut self) -> IonResult<usize> {
    let length = match self.cursor.value.header.length_code {
      0 => 0,
      4 => 4,
      8 => 8,
      LENGTH_CODE_NULL => 0,
      _ => return decoding_error(
        format!(
          "Found a Float value with an illegal length: {}",
          self.cursor.value.header.length_code
        )
      )
    };
    Ok(length)
  }

  fn read_struct_length(&mut self) -> IonResult<usize> {
    let length = match self.cursor.value.header.length_code {
      LENGTH_CODE_NULL => 0,
      1 | LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_next_value_header(&mut self) -> IonResult<Option<IonValueHeader>> {

    let next_byte: u8 = match self.next_byte() {
      Ok(Some(byte)) => byte, // This is the one-byte header of the next value.
      Ok(None) => return Ok(None), // There's no more data to read.
      Err(error) => return Err(error) // Something went wrong while reading the next byte.
    };

    self.header_cache[next_byte as usize].clone()

//    HEADERS.with(|headers| headers[next_byte as usize].clone())
//    SLOW_HEADERS[next_byte as usize].clone()
//    self.header_cache[next_byte as usize].clone()

//    //debug!("Next byte: {} ({:X})", next_byte, next_byte);
//    let (type_code, length_code) = nibbles_from_byte(next_byte);
//    let ion_type_code = IonTypeCode::from(type_code)?;
//    let header = IonValueHeader {
//      ion_type: ion_type_code.as_type()?,
//      ion_type_code,
//      length_code
//    };
//    //debug!("Next header at byte {}: {:?}", self.cursor.bytes_read, &header);
//    Ok(Some(header))
  }

  fn next_byte(&mut self) -> IonResult<Option<u8>> {
    // If the buffer is empty, fill it and check again.
    if self.data_source.buffer().len() == 0 && self.data_source.fill_buf()?.len() == 0 {
      // If the buffer is still empty after filling it, we're out of data.
      return Ok(None);
    }

    // Return the first byte from the buffer.
    let byte: u8 = self.data_source.buffer()[0];
    self.data_source.consume(1);
    self.cursor.bytes_read += 1;

    Ok(Some(byte))
  }

  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
    use std::io;
    if number_of_bytes == 0 {
      return Ok(());
    }
//    println!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);

    let _ = (&mut self.data_source as &mut IonDataSource).skip_bytes(number_of_bytes)?;
    self.cursor.bytes_read += number_of_bytes;
//    println!("After seek, bytes_read: {}", self.cursor.bytes_read);
    Ok(())
  }

//  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//    use std::io;
//    //debug!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);
//
//    let _bytes_copied = io::copy(
//      &mut self.data_source.by_ref().take(number_of_bytes as u64),
//      &mut io::sink()
//    )?;
//
//    self.cursor.bytes_read += number_of_bytes;
//    //debug!("After seek, bytes_read: {}", self.cursor.bytes_read);
//    Ok(())
//  }

  fn skip_current_value(&mut self) -> IonResult<()> {
    if self.cursor.index_at_depth == 0 {
      //debug!("Haven't read the first value yet. Not skipping.");
      Ok(())
    } else {
      let bytes_to_skip = self.cursor.value.last_byte - self.cursor.bytes_read;
      //debug!("Moving to the next value by skipping {} bytes", bytes_to_skip);
      self.skip_bytes(bytes_to_skip)
    }
  }

  fn read_field_id(&mut self) -> IonResult<IonSymbolId> {
    //debug!("Reading the field_id");
    let var_uint = self.read_var_uint()?;
    let field_id = var_uint.value().into();
    //debug!("Found field_id symbol {}", field_id);
    Ok(field_id)
  }

  fn read_annotations(&mut self) -> IonResult<()> {
    //debug!("Reading annotations.");
    let _annotations_and_value_length = self.read_standard_length()?;
    //debug!("Annotations + value length: {}", annotations_and_value_length);
    let annotations_length = self.read_var_uint()?.value();
    //debug!("Annotations length: {}", annotations_length);
    let mut bytes_read: usize = 0;
    while bytes_read < annotations_length {
      let var_uint = self.read_var_uint()?;
      bytes_read += var_uint.size_in_bytes();
      let annotation_symbol_id = var_uint.value().into();
      self.cursor.value.annotations.push(annotation_symbol_id);
    }
    Ok(())
  }

  pub fn step_in(&mut self) -> IonResult<()> {
    use self::IonType::*;
    match self.cursor.value.ion_type {
      Struct => {
        self.cursor.is_in_struct = true;
      },
      List | SExpression => {
        self.cursor.is_in_struct = false;
      },
      _ => panic!("You cannot step into a(n) {:?}", self.cursor.value.ion_type)
    }
//    let parent = self.cursor.value.clone();
//    self.cursor.value.parent_index = Some(Rc::new(parent));
    //debug!("Stepping into parent: {:?}", parent);
    self.cursor.value.parent_index = Some(self.parents.len());
    self.parents.push(self.cursor.value.clone());
    self.cursor.depth += 1;
    self.cursor.index_at_depth = 0;
    Ok(())
  }

  pub fn step_out(&mut self) -> IonResult<()> {
    use std::mem::swap;
    let bytes_to_skip;
//    let mut parent_value;
    { // parent scope
//      let mut parent_index= match self.cursor.value.parent_index {
//        Some(ref mut parent_index) => parent_index,
//        None => panic!("You cannot step out of the root level.")
//      };

      // Remove the last parent from the parents vec
      let mut parent = self.parents
        .pop()
        .expect("You cannot step out of the root level.");

      //debug!("Currently at byte: {}", self.cursor.bytes_read);
      //debug!("Stepping out of parent: {:?}", parent);
      bytes_to_skip = parent.last_byte - self.cursor.bytes_read;
      //debug!("Bytes to skip: {}", bytes_to_skip);

      //parent_value = (**parent_index).clone();
      swap(&mut self.cursor.value, &mut parent);
    }
    // Revert the cursor's current value to be the parent we stepped into.
    // After some bookkeeping, we'll skip enough bytes to move to the end of the parent.


//    swap(&mut self.cursor.value, &mut parent_value);

    // Check to see what the new top of the parents stack is

    if let Some(ref parent) = self.parents.last() {
      self.cursor.is_in_struct = parent.ion_type == IonType::Struct;
    } else {
      self.cursor.is_in_struct = false;
    }

//    if let Some(ref parent) = self.cursor.value.parent_index {
//      self.cursor.is_in_struct = parent.ion_type == IonType::Struct;
//      //debug!("Are we in a Struct? {}", self.cursor.is_in_struct);
//    } else {
//      self.cursor.is_in_struct = false;
//    }
    self.cursor.index_at_depth = self.cursor.value.index_at_depth;
    self.cursor.depth -= 1;
    //debug!("Stepping out by skipping {} bytes.", bytes_to_skip);
    let _ = self.skip_bytes(bytes_to_skip)?;
    //debug!("After stepping out, bytes_read is: {}", self.cursor.bytes_read);
    Ok(())
  }
}