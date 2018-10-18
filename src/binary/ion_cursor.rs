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

use result::{IonResult, IonError, io_error, decoding_error};

use types::ion_type::IonType;

use std::ops::Deref;
use std::ops::DerefMut;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use types::ion_string::IonStringRef;
use types::ion_boolean::IonBoolean;
use types::ion_integer::IonInteger;
use types::ion_float::IonFloat;
use types::ion_symbol::{IonSymbol, IonSymbolId};
use types::ion_blob::IonBlobRef;
use types::ion_clob::IonClobRef;
use types::ion_timestamp::IonTimestamp;
use binary::var_int::VarInt;

use chrono::prelude::*;
use chrono::offset::FixedOffset;
use types::ion_decimal::IonDecimal;
use binary::int::Int;
use bigdecimal::BigDecimal;
use std::io::ErrorKind;

const LENGTH_CODE_NULL: u8 = 15;
const LENGTH_CODE_VAR_UINT: u8 = 14;

const IVM_LENGTH: usize = 4;
const IVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

#[derive(Copy, Clone, Debug)]
struct IonValueHeader {
  ion_type_code: IonTypeCode,
  length_code: u8
}

#[derive(Clone, Debug)]
struct CursorValue {
  ion_type: IonType,
  header: IonValueHeader,
  is_null: bool,
  index_at_depth: usize, // The number of values read so far at this level
  length_in_bytes: usize,
  last_byte: usize,
  field_id: Option<usize>,
//  annotations: SmallVec<[usize; 2]>,
  annotations: Vec<usize>,
  parent: Option<Box<CursorValue>>,
}

impl Default for CursorValue {
  fn default() -> CursorValue {
    CursorValue {
      ion_type: IonType::Null,
      header: IonValueHeader {
        ion_type_code: IonTypeCode::Null,
        length_code: LENGTH_CODE_NULL,
      },
      field_id: None,
      annotations: Vec::new(),
      is_null: true,
      index_at_depth: 0,
      length_in_bytes: 0,
      last_byte: 0,
      parent: None,
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
pub struct BinaryIonCursor<'cursor, R> where R: IonDataSource + 'cursor {
  data_source: &'cursor mut R,
  buffer: Vec<u8>, // Used for individual read() call
  cursor: CursorState,
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

impl <'cursor, R> BinaryIonCursor<'cursor, R> where R: IonDataSource + Seek + 'cursor {
  pub fn checkpoint(&self) -> CursorState {
    self.cursor.clone()
  }

  pub fn restore(&mut self, mut saved_state: CursorState) -> IonResult<()> {
   use std::mem;
    use std::io::{Seek, SeekFrom};
    mem::swap(&mut self.cursor, &mut saved_state);
    (self.data_source as &mut Seek).seek(SeekFrom::Start(self.cursor.bytes_read as u64))?;
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

impl <'cursor, R> BinaryIonCursor<'cursor, R> where R: IonDataSource + 'cursor {

  pub fn is_null(&self) -> bool {
    self.cursor.value.is_null
  }

  pub fn ion_type(&self) -> IonType {
    self.cursor.value.ion_type
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
    let _ = self.read_exact(number_of_bytes)?;

    let value = match number_of_bytes {
      0 => 0f64,
      4 => BigEndian::read_f32(&self.buffer) as f64,
      8 => BigEndian::read_f64(&self.buffer),
      _ => return decoding_error(
        &format!(
          "Encountered an illegal value for a Float length: {}",
          number_of_bytes
        )
      )
    };

    return Ok(Some(IonFloat::from(value)));
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

  pub fn string_ref_value(&mut self) -> IonResult<Option<IonStringRef>> {
    use std::str;
    if self.cursor.value.ion_type != IonType::String || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value(){
      panic!("You cannot read the same string value more than once.");
    }

    let length_in_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(length_in_bytes)?;
    match str::from_utf8(&self.buffer) {
      Ok(utf8_text) => Ok(Some(IonStringRef::from(utf8_text))),
      Err(utf8_error) =>  decoding_error(
        &format!(
          "The requested string was not valid UTF-8: {:?}",
          utf8_error
        )
      )
    }
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

  pub fn blob_ref_value(&mut self) -> IonResult<Option<IonBlobRef>> {
    if self.cursor.value.ion_type != IonType::Blob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same blob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(number_of_bytes)?;
    Ok(Some(From::from(self.buffer.as_ref())))
  }

  pub fn clob_ref_value(&mut self) -> IonResult<Option<IonClobRef>> {
    if self.cursor.value.ion_type != IonType::Clob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same clob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(number_of_bytes)?;
    Ok(Some(From::from(self.buffer.as_ref())))
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

    let total_length = self.cursor.value.length_in_bytes;
    let mut bytes_read = 0;

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
      if self.finished_reading_value() { // TODO: Private method
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

  pub fn annotations<'a>(&'a self) -> impl Iterator<Item=usize> + 'a {
    self.cursor.value.annotations.iter().cloned()
  }

  pub fn field_id(&self) -> Option<IonSymbolId> {
    self.cursor.value.field_id.map(Into::into)
  }

  pub fn value_is_symbol_table(&self) -> bool {
    match (self.ion_type(), self.annotations().next()) {
      (IonType::Struct, Some(3)) => true,
      _ => false
    }
  }

  pub fn new(data_source: &mut R) -> IonResult<BinaryIonCursor<R>> {
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
      data_source,
      buffer: vec![],
      cursor: CursorState {
        bytes_read: IVM_LENGTH,
        depth: 0,
        index_at_depth: 0,
        is_in_struct: false,
        value: Default::default()
      },
    })
  }

  pub fn next(&mut self) -> IonResult<Option<IonType>> {
    let _ = self.skip_current_value()?;

    if let Some(ref parent) = self.cursor.value.parent {
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
      None => return Ok(None)
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
    self.buffer.resize(number_of_bytes, 0); // Will grow the buffer if needed.
    let _ = self.data_source.read_exact(&mut self.buffer)?;
    self.cursor.bytes_read += number_of_bytes;
    Ok(())
  }

  fn process_header_by_type_code(&mut self, header: &IonValueHeader) -> IonResult<()> {
    let length_code = header.length_code;
    let ion_type_code = header.ion_type_code;

    self.cursor.value.ion_type = header.ion_type_code.as_type()?;
    self.cursor.value.header = *header;
    self.cursor.value.is_null = header.length_code == LENGTH_CODE_NULL;

    use self::IonTypeCode::*;
    let length = match ion_type_code {
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
      Reserved => return decoding_error("Found an Ion Value with a Reserved type code."),
       _ => unreachable!("Unexpected IonTypeCode value in header.")
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
        ).as_ref()
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

    //debug!("Next byte: {} ({:X})", next_byte, next_byte);
    let (type_code, length_code) = nibbles_from_byte(next_byte);
    let ion_type_code = IonTypeCode::from(type_code)?;
    let header = IonValueHeader {
      ion_type_code,
      length_code
    };
    //debug!("Next header at byte {}: {:?}", self.cursor.bytes_read, &header);
    Ok(Some(header))
  }

  fn next_byte(&mut self) -> IonResult<Option<u8>> {
    self.cursor.bytes_read += 1;
//    match self.data_source.bytes().next() {
//      None => Ok(None),
//      Some(Ok(byte)) => Ok(Some(byte)),
//      Some(Err(error)) => io_error(error.description())
//    }
    let mut buf = [0];
    loop {
      return match self.data_source.read(&mut buf) {
        Ok(0) => Ok(None),
        Ok(..) => Ok(Some(buf[0])),
        Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
        Err(error) => io_error(error.description()),
      };
    }
  }

  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
    use std::io;
    if number_of_bytes == 0 {
      return Ok(());
    }
//    println!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);

    let _ = (self.data_source as &mut IonDataSource).skip_bytes(number_of_bytes)?;
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

  fn read_field_id(&mut self) -> IonResult<usize> {
    //debug!("Reading the field_id");
    let var_uint = self.read_var_uint()?;
    let field_id = var_uint.value();
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
      let annotation_symbol_id = var_uint.value();
      self.cursor.value.annotations.push(annotation_symbol_id);
    }
    Ok(())
  }

  pub fn step_in(&mut self) -> IonResult<()> {
    use self::IonType::*;
    match self.cursor.value.ion_type {
      List | SExpression => {
        self.cursor.is_in_struct = false;
      },
      Struct => {
        self.cursor.is_in_struct = true;
      },
      _ => panic!("You cannot step into a(n) {:?}", self.cursor.value.ion_type)
    }
    let parent = self.cursor.value.clone();
    //debug!("Stepping into parent: {:?}", parent);
    self.cursor.value.parent = Some(Box::new(parent));
    self.cursor.depth += 1;
    self.cursor.index_at_depth = 0;
    Ok(())
  }

  pub fn step_out(&mut self) -> IonResult<()> {
    use std::mem::swap;
    let bytes_to_skip;
    let mut parent_value;
    { // parent scope
      let parent = match self.cursor.value.parent {
        Some(ref parent) => parent,
        None => panic!("You cannot step out of the root level.")
      };
      //debug!("Currently at byte: {}", self.cursor.bytes_read);
      //debug!("Stepping out of parent: {:?}", parent);
      bytes_to_skip = parent.last_byte - self.cursor.bytes_read;
      //debug!("Bytes to skip: {}", bytes_to_skip);
      parent_value = (**parent).clone();
    }
    // Revert the cursor's current value to be the parent we stepped into.
    // After some bookkeeping, we'll skip enough bytes to move to the end of the parent.
    swap(&mut self.cursor.value, &mut parent_value);
    if let Some(ref parent) = self.cursor.value.parent {
      self.cursor.is_in_struct = parent.ion_type == IonType::Struct;
      //debug!("Are we in a Struct? {}", self.cursor.is_in_struct);
    } else {
      self.cursor.is_in_struct = false;
    }
    self.cursor.index_at_depth = self.cursor.value.index_at_depth;
    self.cursor.depth -= 1;
    //debug!("Stepping out by skipping {} bytes.", bytes_to_skip);
    let _ = self.skip_bytes(bytes_to_skip)?;
    //debug!("After stepping out, bytes_read is: {}", self.cursor.bytes_read);
    Ok(())
  }
}