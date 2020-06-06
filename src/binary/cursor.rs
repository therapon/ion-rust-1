use bytes::BigEndian;
use bytes::ByteOrder;

use std::io::Seek;

use chrono::prelude::*;
use chrono::offset::FixedOffset;

use bigdecimal::BigDecimal;

use crate::{
  data_source::IonDataSource,
  binary::{
    int::Int,
    uint::UInt,
    var_int::VarInt,
    var_uint::VarUInt,
    header::{
      Header,
      create_header_byte_jump_table
    },
    IonTypeCode,
    constants::v1_0::{
      IVM,
      length_codes
    }
  },
  result::{IonResult, decoding_error},
  types::{IonType, SymbolId},
};
use crate::cursor::{Cursor, StreamItem};
use crate::binary::constants::v1_0::system_symbol_ids;

#[derive(Clone, Debug)]
struct CursorValue {
  ion_type: IonType, // TODO: Eliminate in favor of ion_type in header?
  header: Header,
  is_null: bool,
  index_at_depth: usize, // The number of values read so far at this level
  length_in_bytes: usize,
  last_byte: usize,
  field_id: Option<SymbolId>,
  annotations: Vec<SymbolId>,
  parent_index: Option<usize>
}

impl Default for CursorValue {
  fn default() -> CursorValue {
    CursorValue {
      ion_type: IonType::Null,
      header: Header {
        ion_type: None,
        ion_type_code: IonTypeCode::NullOrWhitespace,
        length_code: length_codes::NULL,
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

// A low-level reader that offers no validation or symbol management.
// It can only move and return the current value.
pub struct BinaryIonCursor<R> where R: IonDataSource {
  data_source: R,
  buffer: Vec<u8>, // Used for individual data_source.read() calls independent of input buffering
  cursor: CursorState,
  header_cache: Vec<IonResult<Option<Header>>>
}

/* CursorState is broken out from the BinaryIonCursor struct to allow it to be cloned
 * or replaced as part of a seek operation.
 */
#[derive(Clone, Debug)]
// TODO: Rename as part of the checkpoint/restore APIs
pub struct CursorState {
  ion_version: (u8, u8),
  bytes_read: usize, // How many bytes we've read from `data_source`
  depth: usize, // How deeply nested the cursor is at the moment
  index_at_depth: usize, // The number of values (starting with 0) read at the current depth
  is_in_struct: bool, // Whether this level of descent is within a struct
  value: CursorValue, // Information about the value on which the cursor is currently sitting.
  parents: Vec<CursorValue>,
}

// //TODO Move other I/O functions here? Remove all I/O functions and just depend on BufRead?
// pub trait IonDataSource: BufRead {
//   fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()>;
// }
//
// impl <T: BufRead> IonDataSource for T {
// //impl <T> IonDataSource for T where T: BufRead {
//   default fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//     use std::io;
//     let _bytes_copied = io::copy(
//       &mut self.by_ref().take(number_of_bytes as u64),
//       &mut io::sink()
//     )?;
//     Ok(())
//   }
// }
//
// impl <T: BufRead + AsRef<[u8]>> IonDataSource for io::Cursor<T> {
//   fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
//     self.set_position(self.position() + number_of_bytes as u64);
//     Ok(())
//   }
// }

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

//impl <R> BinaryIonCursor<io::Cursor<R>> where R: IonDataSource + AsRef<[u8]> {
//  pub fn region_cursor(&mut self, start: usize, end: usize) -> Self {
//    let state = self.checkpoint();
//    let io_cursor = &self.data_source.clone();
//    let mut region_cursor = BinaryIonCursor::unsafe_new(io_cursor);
//    region_cursor.restore(state);
//    region_cursor
//  }
//}

impl <R> BinaryIonCursor<R> where R: IonDataSource + Seek {
  pub fn checkpoint(&self) -> CursorState {
    self.cursor.clone()
  }

  pub fn restore(&mut self, mut saved_state: CursorState) -> IonResult<CursorState> {
    use std::{
      mem,
      io::SeekFrom
    };
    mem::swap(&mut self.cursor, &mut saved_state);
    (&mut self.data_source).seek(SeekFrom::Start(self.cursor.bytes_read as u64))?;
    Ok(saved_state)
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

//struct BinaryIonCursorPositions<'cursor, R: IonDataSource> {
//  cursor: &'cursor mut BinaryIonCursor<R>
//}
//
//impl <'cursor, R: IonDataSource> Iterator for BinaryIonCursorPositions<'cursor, R> {
//  type Item = IonResult<(IonType, &'cursor mut BinaryIonCursor<R>)>;
//
//  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
//    let ion_type = match self.cursor.next() {
//      Ok(Some(ion_type)) => ion_type,
//      Ok(None) => return None,
//      Err(error) => return Some(Err(error))
//    };
//    Some(Ok((ion_type, self.cursor)))
//  }
//}

// TODO: Macro-ize type checks and already-read checks
macro_rules! read_type_check {
  ( $binary_cursor:ident, $ion_type:expr ) => {
    if $binary_cursor.cursor.value.ion_type != $ion_type || $binary_cursor.cursor.value.is_null {
      return Ok(None);
    }
  }
}

/// Verifies that the current value is of the expected type and that the bytes representing that
/// value have not yet been consumed.
macro_rules! read_safety_checks {
  ( $binary_cursor:ident, $ion_type:expr ) => {
    read_type_check!($binary_cursor, $ion_type);
    if $binary_cursor.cursor.value.length_in_bytes > 0 && $binary_cursor.finished_reading_value() {
      panic!(format!("You cannot read the same {:?} value more than once.", $ion_type));
    }
  }
}

impl <R> BinaryIonCursor<R> where R: IonDataSource {
  fn value_is_symbol_table(&self) -> bool {
    match (self.ion_type(), self.cursor.value.annotations.as_slice()) {
      (Some(IonType::Struct), [system_symbol_ids::ION_SYMBOL_TABLE, ..])  => true,
      _ => false
    }
  }

  //TODO: This can just return Self if we defer reading the IVM.
  //We should do this anyway because reading the IVM should happen inside of next().
  //Any number of IVMs can appear in the input and we need to reset the symbol table when this
  //occurs.
  pub fn new(mut data_source: R) -> IonResult<Self> {
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
      data_source: data_source,
      buffer: vec![0; 1024],
      cursor: CursorState {
        ion_version: (1, 0),
        bytes_read: IVM.len(),
        depth: 0,
        index_at_depth: 0,
        is_in_struct: false,
        value: Default::default(),
        parents: Vec::new(),
      },
      header_cache: create_header_byte_jump_table()
    })
  }

  // not really unsafe, just doesn't attempt to read() yet, so it won't tell you whether the
  // data source is really binary Ion.
  pub fn unsafe_new(data_source: R) -> Self {
    BinaryIonCursor {
      data_source: data_source,
      buffer: vec![0; 1024],
      cursor: CursorState {
        ion_version: (1, 0),
        bytes_read: 0,
        depth: 0,
        index_at_depth: 0,
        is_in_struct: false,
        value: Default::default(),
        parents: Vec::new(),
      },
      header_cache: create_header_byte_jump_table()
    }
  }


  fn depth(&self) -> usize {
    self.cursor.depth
  }

  fn is_null(&self) -> bool {
    self.cursor.value.is_null
  }

  fn finished_reading_value(&mut self) -> bool {
    self.cursor.bytes_read >= self.cursor.value.last_byte
  }

  fn read_var_uint(&mut self) -> IonResult<VarUInt> {
    let var_uint = VarUInt::read(&mut self.data_source)?;
    self.cursor.bytes_read += var_uint.size_in_bytes();
    Ok(var_uint)
  }

  fn read_var_int(&mut self) -> IonResult<VarInt> {
    let var_int = VarInt::read(&mut self.data_source)?;
    self.cursor.bytes_read += var_int.size_in_bytes() as usize;
    Ok(var_int)
  }

  // Useful when the entire value (all bytes after the type/length header) constitute a single UInt
  fn read_value_as_uint(&mut self) -> IonResult<UInt> {
    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.read_uint(number_of_bytes)
  }

  fn read_uint(&mut self, number_of_bytes: usize) -> IonResult<UInt> {
    let uint = UInt::read(&mut self.data_source, number_of_bytes)?;
    self.cursor.bytes_read += uint.size_in_bytes();
    Ok(uint)
  }

  fn read_int(&mut self, number_of_bytes: usize) -> IonResult<Int> {
    let int = Int::read(&mut self.data_source, number_of_bytes)?;
    self.cursor.bytes_read += int.size_in_bytes();
    Ok(int)
  }

  fn process_header_by_type_code(&mut self, header: Header) -> IonResult<()> {

//    self.cursor.value.ion_type = header.ion_type_code.as_type()?;
    self.cursor.value.ion_type = header.ion_type.unwrap();
    self.cursor.value.header = header;
    self.cursor.value.is_null = header.length_code == length_codes::NULL;

    use IonTypeCode::*;
    let length = match header.ion_type_code {
      NullOrWhitespace
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
      length_codes::NULL => 0,
      length_codes::VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_float_length(&mut self) -> IonResult<usize> {
    let length = match self.cursor.value.header.length_code {
      0 => 0,
      4 => 4,
      8 => 8,
      length_codes::NULL => 0,
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
      length_codes::NULL => 0,
      1 | length_codes::VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_next_value_header(&mut self) -> IonResult<Option<Header>> {

    let next_byte: u8 = match self.next_byte() {
      Ok(Some(byte)) => byte, // This is the one-byte header of the next value.
      Ok(None) => return Ok(None), // There's no more data to read.
      Err(error) => return Err(error) // Something went wrong while reading the next byte.
    };

    self.header_cache[next_byte as usize].clone()
  }

  fn next_byte(&mut self) -> IonResult<Option<u8>> {
    // If the buffer is empty, fill it and check again.
    let buffer = self.data_source.fill_buf()?;
    if buffer.is_empty() {
      // If the buffer is still empty after filling it, we're out of data.
      return Ok(None);
    }

    // Return the first byte from the buffer.
    let byte: u8 = buffer[0];
    self.data_source.consume(1);
    self.cursor.bytes_read += 1;

    Ok(Some(byte))
  }

  fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
    if number_of_bytes == 0 {
      return Ok(());
    }
//    println!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);

    // (&mut self.data_source as &mut IonDataSource).skip_bytes(number_of_bytes)?;
    &mut self.data_source.skip_bytes(number_of_bytes)?;
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

  fn read_field_id(&mut self) -> IonResult<SymbolId> {
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

    self.data_source.read_exact(buffer)?;
    self.cursor.bytes_read += number_of_bytes;
    Ok(())
  }

  fn parse_n_bytes<T, F>(&mut self, number_of_bytes: usize, processor: F) -> IonResult<T>
    where F: FnOnce(&[u8]) -> IonResult<T> {

    // If the requested value is already in our input buffer, there's no need to copy it out into a
    // separate buffer. We can return a slice of the input buffer and consume() that number of
    // bytes.

    let buffer = self.data_source.fill_buf()?;

    if buffer.len() >= number_of_bytes {
//      println!("We have {} bytes, we need {} bytes.", self.data_source.buffer().len(), number_of_bytes);
      let result = processor(&buffer[..number_of_bytes]);
      self.data_source.consume(number_of_bytes);
      self.cursor.bytes_read += number_of_bytes;
      return result;
    }

    // Otherwise, read the value into self.buffer, a growable Vec.
    // It would be nice to use the buffer Vec as-is when it has enough capacity. Unfortunately,
    // Vec doesn't allow you to get a reference to its underlying [T] array beyond .len().
    // Thus, we can only optimize this for the case where the vec's .len() is larger than what we need.
    // As a minor hack, we can ensure the vec is always full of 0s so that len() == capacity.

    // Grow the Vec if needed
    let buffer: &mut [u8] = if self.buffer.len() < number_of_bytes {
      self.buffer.resize(number_of_bytes, 0);
      self.buffer.as_mut()
    } else {
      // Otherwise, split the Vec's underlying storage to get a &mut [u8] slice of the required size
      let (required_buffer, _) = self.buffer.split_at_mut(number_of_bytes);
      required_buffer
    };

    self.data_source.read_exact(buffer)?;
    let result = processor(buffer);
    self.cursor.bytes_read += number_of_bytes;
    result
  }

  pub fn blob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&[u8]) -> T {
    if self.cursor.value.ion_type != IonType::Blob || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same blob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      Ok(Some(f(buffer)))
    })
  }

  pub fn clob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&[u8]) -> T {
    if self.cursor.value.ion_type != IonType::Clob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same clob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      Ok(Some(f(buffer)))
    })
  }

  pub fn string_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&str) -> T {
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
        Ok(utf8_text) => utf8_text,
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
}

impl <R: IonDataSource> Cursor<R> for BinaryIonCursor<R> {
  fn next(&mut self) -> IonResult<Option<StreamItem>> {
    let _ = self.skip_current_value()?;

    if let Some(ref parent) = self.cursor.parents.last() {
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
      if header.length_code == 0 {
        // It's a Version Marker
        // TODO: actually parse the IVM
        self.cursor.ion_version = (1, 0);
        self.data_source.skip_bytes(3)?;
        return Ok(Some(StreamItem::VersionMarker));
      }
      // We've found an annotated value. Read all of the annotation symbols leading up to the value.
      let _ = self.read_annotations()?;
      // Now read the next header representing the value itself.
      header = match self.read_next_value_header()? {
        Some(header) => header,
        None => return Ok(None)
      };
      self.cursor.value.header = header;
    }

    let _ = self.process_header_by_type_code(header)?;

    self.cursor.index_at_depth += 1;
    self.cursor.value.index_at_depth = self.cursor.index_at_depth;

    // Differentiate between a struct and a system-level struct
    if self.value_is_symbol_table() {
      return Ok(Some(StreamItem::SymbolTableImport))
    }

    Ok(Some(StreamItem::Value(self.cursor.value.ion_type, self.is_null())))
  }

  fn ion_type(&self) -> Option<IonType> {
    // TODO: Handle cases where this should be None:
    // * Haven't called next() yet so there's no current value
    // * Positioned over a symbol table
    // * Positioned over an IVM
    Some(self.cursor.value.ion_type)
  }

  fn annotation_ids(&self) -> &[SymbolId] {
    &self.cursor.value.annotations
  }

  fn field_id(&self) -> Option<SymbolId> {
    self.cursor.value.field_id
  }

  fn read_null(&mut self) -> IonResult<Option<IonType>> {
    if self.is_null() {
      return Ok(Some(self.cursor.value.ion_type));
    }
    Ok(None)
  }

  fn read_bool(&mut self) -> IonResult<Option<bool>> {
    if self.cursor.value.ion_type != IonType::Boolean || self.cursor.value.is_null {
      return Ok(None);
    }

    // No reading from the stream occurs -- the header contains all of the information we need.
    let representation = self.cursor.value.header.length_code;

    match representation {
      0 => Ok(Some(false)),
      1 => Ok(Some(true)),
      _ => decoding_error(
        &format!("Found a boolean value with an illegal representation: {}", representation)
      )
    }
  }

  // TODO:
  // - Detect overflow and return None
  // - Add an integer_size() method that indicates whether the current value will
  //   fit in an i32, i64, i128 or BigInteger
  fn read_i64(&mut self) -> IonResult<Option<i64>> {
    // if self.cursor.value.ion_type != IonType::Integer ||  self.cursor.value.is_null {
    //   return Ok(None);
    // }
    // if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
    //   panic!("You cannot read the same integer value more than once.");
    // }
    // read_safety_check!(self, IonType::Integer);
    read_safety_checks!(self, IonType::Integer);

    let magnitude = self.read_value_as_uint()?.value();

    use self::IonTypeCode::*;
    let value = match self.cursor.value.header.ion_type_code {
      PositiveInteger => magnitude as i64,
      NegativeInteger => -(magnitude as i64),
      _ => unreachable!("The Ion Type Code must be one of the above to reach this point.")
    };

    Ok(Some(value))
  }

  //TODO: Currently this just calls read_f64 and then casts to f32. Is there a better behavior?
  fn read_f32(&mut self) -> IonResult<Option<f32>> {
    match self.read_f64() {
      Ok(Some(value)) => Ok(Some(value as f32)), // Lossy if the value was 64 bits
      Ok(None) => Ok(None),
      Err(e) => Err(e)
    }
  }

  fn read_f64(&mut self) -> IonResult<Option<f64>> {
    if self.cursor.value.ion_type != IonType::Float || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same float value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;

    self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| {
      let value = match number_of_bytes {
        0 => 0f64,
        4 => f64::from(BigEndian::read_f32(buffer)),
        8 => BigEndian::read_f64(buffer),
        _ => return decoding_error(
          &format!(
            "Encountered an illegal value for a Float length: {}",
            number_of_bytes
          )
        )
      };
      Ok(Some(value))
    })
  }

  fn read_big_decimal(&mut self) -> IonResult<Option<BigDecimal>> {
    if self.cursor.value.ion_type != IonType::Decimal ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same decimal value more than once.");
    }

    if self.cursor.value.length_in_bytes == 0 {
      return Ok(Some(BigDecimal::new(0i64.into(), 0)));
    }

    let exponent_var_int = self.read_var_int()?;
    let coefficient_size_in_bytes = self.cursor.value.length_in_bytes - exponent_var_int.size_in_bytes();

    let exponent = exponent_var_int.value() as i64;
    let coefficient = self.read_int(coefficient_size_in_bytes)?.value();

    // BigDecimal uses 'scale' rather than 'exponent' in its API, which is a count of the number of
    // decimal places. It's effectively `exponent * -1`.
    Ok(Some(BigDecimal::new(coefficient.into(), -exponent)))
  }

  fn read_string(&mut self) -> IonResult<Option<String>> {
    self.string_ref_map(|s: &str| s.into())
  }

  fn read_symbol_id(&mut self) -> IonResult<Option<SymbolId>> {
    if self.cursor.value.ion_type != IonType::Symbol || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same symbol ID value more than once.");
    }

    let symbol_id = self.read_value_as_uint()?.value() as usize;
    Ok(Some(symbol_id))
  }

  fn read_blob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
    self.blob_ref_map(|b| b.into())
  }

  fn read_clob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
    self.clob_ref_map(|c| c.into())
  }

  fn read_datetime(&mut self) -> IonResult<Option<DateTime<FixedOffset>>> {
    if self.cursor.value.ion_type != IonType::Timestamp || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
      panic!("You cannot read the same timestamp value more than once.");
    }

    let offset_minutes = self.read_var_int()?.value();
    let year = self.read_var_uint()?.value();

    let mut month = 0;
    let mut day = 0;
    let mut hour = 0;
    let mut minute = 0;
    let mut second = 0;

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

      // TODO: Fractional seconds decimal
    }

    let naive_datetime =
      NaiveDate::from_ymd(year as i32, month as u32, day as u32)
                .and_hms(hour as u32, minute as u32, second as u32);
    let offset = FixedOffset::west(offset_minutes as i32 * 60i32);
    let datetime = offset.from_utc_datetime(&naive_datetime);
    Ok(Some(datetime))
  }

  fn step_in(&mut self) -> IonResult<()> {
    use self::IonType::*;
    self.cursor.is_in_struct = match self.cursor.value.ion_type {
      Struct => true,
      List | SExpression => false,
      _ => panic!("You cannot step into a(n) {:?}", self.cursor.value.ion_type)
    };
    self.cursor.value.parent_index = Some(self.cursor.parents.len());
    self.cursor.parents.push(self.cursor.value.clone());
    self.cursor.depth += 1;
    self.cursor.index_at_depth = 0;
    Ok(())
  }

  fn step_out(&mut self) -> IonResult<()> {
    use std::mem;
    let bytes_to_skip;

    // Remove the last parent from the parents vec
    let mut parent = self.cursor.parents
      .pop()
      .expect("You cannot step out of the root level.");

    // We're stepping out of the container, so we need to skip to the end of it.
    bytes_to_skip = parent.last_byte - self.cursor.bytes_read;

    // Set the parent as the current value
    mem::swap(&mut self.cursor.value, &mut parent);

    // Check to see what the new top of the parents stack is
    if let Some(ref parent) = self.cursor.parents.last() {
      self.cursor.is_in_struct = parent.ion_type == IonType::Struct;
    } else {
      self.cursor.is_in_struct = false;
    }

    self.cursor.index_at_depth = self.cursor.value.index_at_depth;
    self.cursor.depth -= 1;
    self.skip_bytes(bytes_to_skip)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::binary::constants::v1_0::IVM;
  use std::io;
  use crate::binary::cursor::BinaryIonCursor;
  use crate::types::IonType;
  use crate::result::IonResult;
  use crate::cursor::{StreamItem::*, Cursor};
  use bigdecimal::BigDecimal;
  use chrono::{FixedOffset, NaiveDate, TimeZone};

  type TestDataSource = io::Cursor<Vec<u8>>;

  // Create a growable byte vector that starts with the 1.0 IVM
  fn ion_data(bytes: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&IVM);
    data.extend_from_slice(bytes);
    data
  }

  fn data_source_for(bytes: &[u8]) -> TestDataSource {
    let data = ion_data(bytes);
    io::Cursor::new(data)
  }

  fn ion_cursor_for(bytes: &[u8]) -> BinaryIonCursor<TestDataSource> {
    let mut binary_cursor = BinaryIonCursor::new(data_source_for(bytes));
    assert_eq!(binary_cursor.next(), Ok(Some(VersionMarker)));
    assert_eq!(binary_cursor.ion_v)
    binary_cursor
  }

  #[test]
  fn test_read_null_null() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x0F]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Null, true)));
    assert_eq!(cursor.read_null()?, Some(IonType::Null));
    assert!(cursor.is_null());
    Ok(())
  }

  #[test]
  fn test_read_null_string() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x8F]);
    assert_eq!(cursor.next()?, Some(Value(IonType::String, true)));
    assert_eq!(cursor.read_null()?, Some(IonType::String));
    assert!(cursor.is_null());
    Ok(())
  }

  #[test]
  fn test_read_bool_false() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x10]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Boolean, false)));
    assert_eq!(cursor.read_bool()?, Some(false));
    Ok(())
  }

  #[test]
  fn test_read_bool_true() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x11]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Boolean, false)));
    assert_eq!(cursor.read_bool()?, Some(true));
    Ok(())
  }

  #[test]
  fn test_read_i64_zero() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x20]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.read_i64()?, Some(0i64));
    Ok(())
  }

  #[test]
  fn test_read_i64_positive() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x21, 0x01]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.read_i64()?, Some(1i64));
    Ok(())
  }

  #[test]
  fn test_read_i64_negative() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x31, 0x01]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.read_i64()?, Some(-1i64));
    Ok(())
  }

  #[test]
  fn test_read_f64_zero() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x40]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Float, false)));
    assert_eq!(cursor.read_f64()?, Some(0f64));
    Ok(())
  }

  #[test]
  fn test_read_big_decimal_zero() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x50]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Decimal, false)));
    assert_eq!(cursor.read_big_decimal()?, Some(BigDecimal::new(0i64.into(), 0)));
    Ok(())
  }

  #[test]
  fn test_read_big_decimal_positive_exponent() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x52, 0x81, 0x02]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Decimal, false)));
    assert_eq!(cursor.read_big_decimal()?, Some(20.into()));
    Ok(())
  }

  #[test]
  fn test_read_big_decimal_negative_exponent() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x52, 0xC1, 0x02]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Decimal, false)));
    assert_eq!(cursor.read_big_decimal()?, Some(0.2f64.into()));
    Ok(())
  }

  #[test]
  fn test_read_timestamp() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x68, 0x80, 0x0F, 0xD0, 0x81, 0x81, 0x80, 0x80, 0x80]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Timestamp, false)));
    let naive_datetime =
        NaiveDate::from_ymd(2000 as i32, 1 as u32, 1 as u32)
                  .and_hms(0 as u32, 0 as u32, 0 as u32);
    let offset = FixedOffset::west(0);
    let datetime = offset.from_utc_datetime(&naive_datetime);
    assert_eq!(cursor.read_datetime()?, Some(datetime));
    Ok(())
  }

  #[test]
  fn test_read_symbol_10() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x71, 0x0A]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Symbol, false)));
    assert_eq!(cursor.read_symbol_id()?, Some(10usize));
    Ok(())
  }

  #[test]
  fn test_read_string_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x80]);
    assert_eq!(cursor.next()?, Some(Value(IonType::String, false)));
    assert_eq!(cursor.read_string()?, Some(String::from("")));
    Ok(())
  }

  #[test]
  fn test_read_string_foo() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x83, 0x66, 0x6f, 0x6f]);
    assert_eq!(cursor.next()?, Some(Value(IonType::String, false)));
    assert_eq!(cursor.read_string()?, Some(String::from("foo")));
    Ok(())
  }

  #[test]
  fn test_read_clob_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x90]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Clob, false)));
    assert_eq!(cursor.read_clob_bytes()?, Some(vec![]));
    Ok(())
  }

  #[test]
  fn test_read_clob_abc() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0x93, 0x61, 0x62, 0x63]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Clob, false)));
    assert_eq!(cursor.read_clob_bytes()?.unwrap().as_slice(), "abc".as_bytes());
    Ok(())
  }

  #[test]
  fn test_read_blob_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xA0]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Blob, false)));
    assert_eq!(cursor.read_blob_bytes()?, Some(vec![]));
    Ok(())
  }

  #[test]
  fn test_read_blob_123() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xA3, 0x01, 0x02, 0x03]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Blob, false)));
    assert_eq!(cursor.read_blob_bytes()?, Some(vec![1u8, 2, 3]));
    Ok(())
  }

  #[test]
  fn test_read_list_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xB0]);
    assert_eq!(cursor.next()?, Some(Value(IonType::List, false)));
    cursor.step_in()?;
    assert_eq!(cursor.next()?, None);
    cursor.step_out()?;
    Ok(())
  }

  #[test]
  fn test_read_list_123() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xB6, 0x21, 0x01, 0x21, 0x02, 0x21, 0x03]);
    assert_eq!(cursor.next()?, Some(Value(IonType::List, false)));
    let mut list = vec![];
    cursor.step_in()?;
    while let Some(Value(IonType::Integer, false)) = cursor.next()? {
      list.push(cursor.read_i64()?.unwrap());
    }
    cursor.step_out()?;
    assert_eq!(list, vec![1i64, 2, 3]);
    Ok(())
  }

  #[test]
  fn test_read_s_expression_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xC0]);
    assert_eq!(cursor.next()?, Some(Value(IonType::SExpression, false)));
    cursor.step_in()?;
    assert_eq!(cursor.next()?, None);
    cursor.step_out()?;
    Ok(())
  }

  #[test]
  fn test_read_s_expression_123() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xC6, 0x21, 0x01, 0x21, 0x02, 0x21, 0x03]);
    assert_eq!(cursor.next()?, Some(Value(IonType::SExpression, false)));
    let mut sexp = vec![];
    cursor.step_in()?;
    while let Some(Value(IonType::Integer, false)) = cursor.next()? {
      sexp.push(cursor.read_i64()?.unwrap());
    }
    cursor.step_out()?;
    assert_eq!(sexp, vec![1i64, 2, 3]);
    Ok(())
  }

  #[test]
  fn test_read_struct_empty() -> IonResult<()> {
    let mut cursor = ion_cursor_for(&[0xD0]);
    assert_eq!(cursor.next()?, Some(Value(IonType::Struct, false)));
    cursor.step_in()?;
    assert_eq!(cursor.next()?, None);
    cursor.step_out()?;
    Ok(())
  }

  #[test]
  fn test_read_struct() -> IonResult<()> {
    // TODO: This is technically not valid Ion as the symbol IDs being used have not been added to
    //       the symbol table. The Cursor doesn't attempt to resolve them, so no error is raised.
    let mut cursor = ion_cursor_for(
      &[
        0xD9, // 9-byte struct
        0x8A, // Field ID 10
        0x21, 0x01, // Integer 1
        0x8B, // Field ID 11
        0x21, 0x02, // Integer 2
        0x8C, // Field ID 12
        0x21, 0x03, // Integer 3
      ]
    );
    assert_eq!(cursor.next()?, Some(Value(IonType::Struct, false)));
    cursor.step_in()?;
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.field_id(), Some(10usize));
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.field_id(), Some(11usize));
    assert_eq!(cursor.next()?, Some(Value(IonType::Integer, false)));
    assert_eq!(cursor.field_id(), Some(12usize));
    cursor.step_out()?;
    Ok(())
  }
}