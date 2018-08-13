use std::error::Error;
use std::io::{Read, Seek};
use std::collections::vec_deque::VecDeque;

use bytes::BigEndian;
use bytes::ByteOrder;

use header_byte::*;
use var_uint::VarUInt;
use ion_type::IonType;
use ion_type_code::IonTypeCode;
use errors::{IonError, io_error, decoding_error};
use uint::UInt;
use std::ops::Deref;
use std::ops::DerefMut;

const LENGTH_CODE_NULL: u8 = 15;
const LENGTH_CODE_VAR_UINT: u8 = 14;

const IVM_LENGTH: usize = 4;
const IVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

#[cfg(test)]
mod tests {
  use std::fs::File;
  use ion_type::IonType;
  use super::BinaryIonCursor;
  use errors::IonError;
  use std::io::Read;
  use std::io::BufReader;

  #[test]
  fn test1() {
    //env_logger::init();
//    let path = "/Users/zslayton/local_ion_data/ion_data2/annotated_values.10n";
    let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
    match skim_file(path) {
      Ok(_) => {},
      Err(error) => error!("Failed to read the file: {:?}", error)
    }
  }

  fn skim_file(path: &str) -> Result<(), IonError> {
    let file = File::open(path).expect("Unable to open file");
    let mut reader = BufReader::with_capacity(512 * 1_024, file);
    let mut cursor = BinaryIonCursor::new(&mut reader)?;
    skim_values(&mut cursor, 0)
  }

  fn skim_values<R: Read>(cursor: &mut BinaryIonCursor<R>, depth: usize) -> Result<(), IonError> {
//    let mut marker: String = String::new();
//    marker.push_str("\\");
//    marker.push_str(&"-".repeat(depth + 1));
//    marker.push_str(">");
    let mut current_ion_type: Option<IonType> = cursor.next()?;
    while let Some(ion_type) = current_ion_type {
      { // Annotations scope
        let annotations = cursor.annotations();
        let num_annotations = annotations.len();
        let field_id = cursor.field_id();
        match (num_annotations, field_id) {
          (0, Some(ref field_id)) => {
            //debug!("{} Found a(n) {:?} with field_id: {:?}", marker, ion_type, field_id);
          },
          (0, None) => {
            //debug!("{} Found a(n) {:?}", marker, ion_type);
          },
          (_, Some(ref field_id)) => {
            //debug!("{} Found a(n) {:?} with field_id: {:?} and annotations: {:?}",
//                     marker,
//                     ion_type,
//                     field_id,
//                     annotations);
          },
          (_, None) => {
            //debug!("{} Found a(n) {:?} with annotations: {:?}",
//                     marker,
//                     ion_type,
//                     annotations);
          },
        }
      }

      if cursor.is_null() {
        //debug!("  VALUE: null.{:?}", ion_type);
      } else {
        use self::IonType::*;
        match ion_type {
          String => {
            let text = cursor.string_value()?.unwrap();
            //debug!("  VALUE: {}", text);
          },
          Symbol => {
            let symbol_id = cursor.symbol_value()?.unwrap();
            //debug!("  VALUE: {}", symbol_id);
          },
          Integer => {
            let int = cursor.integer_value()?.unwrap();
            //debug!("  VALUE: {}", int)
          },
          Boolean => {
            let boolean = cursor.boolean_value()?.unwrap();
            //debug!("  VALUE: {}", boolean)
          },
          Float => {
            let float = cursor.float_value()?.unwrap();
            //debug!("  VALUE: {}", float)
          },
          Blob => {
            let blob = cursor.blob_value()?.unwrap();
          },
          Clob => {
            let clob = cursor.clob_value()?.unwrap();
          }
          _ => {
            //debug!("  VALUE: <{:?} not yet supported>", ion_type);
          }
        }
      }

      if ion_type == IonType::List || ion_type == IonType::Struct {
        cursor.step_in()?;
        {
          let _ = skim_values(cursor, depth + 1)?;
        }
        cursor.step_out()?;
      }
      current_ion_type = cursor.next()?;
    }
    Ok(())
  }

}

#[derive(Copy, Clone, Debug)]
struct IonValueHeader {
  ion_type_code: IonTypeCode,
  length_code: u8
}

#[derive(Clone, Debug)]
struct CursorValue {
  ion_type: IonType,
  header: IonValueHeader,
  field_id: Option<usize>,
  annotations: Vec<usize>,
  is_null: bool,
  index_at_depth: usize, // The number of values read so far at this level
  length_in_bytes: usize,
  last_byte: usize,
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
      annotations: vec![],
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

pub struct CursorState {
  bytes_read: usize, // How many bytes we've read from `data_source`
  depth: usize, // How deeply nested the cursor is at the moment
  index_at_depth: usize, // The number of values (starting with 0) read at the current depth
  is_in_struct: bool, // Whether this level of descent is within a struct
  value: CursorValue // Information about the value on which the cursor is currently sitting.
}

// A low-level reader that offers no validation or symbol management.
// It can only move and return the current value.
pub struct BinaryIonCursor<'cursor, R> where R: Read + 'cursor {
  data_source: &'cursor mut R,
  buffer: Vec<u8>, // Used for individual read() call
  cursor: CursorState,
}

impl <'cursor, R> BinaryIonCursor<'cursor, R> where R: Read + 'cursor {

  pub fn is_null(&self) -> bool {
    self.cursor.value.is_null
  }

  pub fn integer_value(&mut self) -> Result<Option<i64>, IonError> {
    use self::IonTypeCode::*;
    if self.cursor.value.ion_type != IonType::Integer ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0 {
      panic!("You cannot read the same integer value more than once.");
    }

    debug!("Length in bytes: {}", self.cursor.value.length_in_bytes);
    let magnitude = self.read_uint()?.value();
    debug!("Magnitude: {}", magnitude);
    let value = match self.cursor.value.header.ion_type_code {
      PositiveInteger => magnitude as i64,
      NegativeInteger => magnitude as i64 * -1,
      _ => unreachable!("The Ion Type Code must be one of the above to reach this point.")
    };

    return Ok(Some(value));
  }

  pub fn float_value(&mut self) -> Result<Option<f64>, IonError> {
    if self.cursor.value.ion_type != IonType::Float ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0 {
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

    return Ok(Some(value));
  }

  pub fn boolean_value(&mut self) -> Result<Option<bool>, IonError> {
    if self.cursor.value.ion_type != IonType::Boolean ||  self.cursor.value.is_null {
      return Ok(None);
    }

    // No reading from the stream occurs -- the header contained all of the information we needed.

    let representation = self.cursor.value.header.length_code;

    match representation {
      0 => Ok(Some(false)),
      1 => Ok(Some(true)),
      _ => decoding_error(
        &format!("Found a boolean value with an illegal representation: {}", representation)
      )
    }
  }

  pub fn string_value(&mut self) -> Result<Option<&str>, IonError> {
    use std::str;
    if self.cursor.value.ion_type != IonType::String || self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0{
      panic!("You cannot read the same string value more than once.");
    }

    let length_in_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(length_in_bytes)?;
    match str::from_utf8(&self.buffer) {
      Ok(string) => Ok(Some(string)),
      Err(utf8_error) =>  decoding_error(
        &format!(
          "The requested string was not valid UTF-8: {:?}",
          utf8_error
        )
      )
    }
  }

  pub fn symbol_value(&mut self) -> Result<Option<usize>, IonError> {
    if self.cursor.value.ion_type != IonType::Symbol ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0 {
      panic!("You cannot read the same symbol value more than once.");
    }

    let symbol_id = self.read_uint()?.value() as usize;
    Ok(Some(symbol_id))
  }

  pub fn blob_value(&mut self) -> Result<Option<&[u8]>, IonError> {
    if self.cursor.value.ion_type != IonType::Blob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0 {
      panic!("You cannot read the same blob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(number_of_bytes)?;
    Ok(Some(&self.buffer))
  }

  pub fn clob_value(&mut self) -> Result<Option<&[u8]>, IonError> {
    if self.cursor.value.ion_type != IonType::Clob ||  self.cursor.value.is_null {
      return Ok(None);
    }

    if self.cursor.bytes_read >= self.cursor.value.last_byte && self.cursor.value.length_in_bytes > 0 {
      panic!("You cannot read the same clob value more than once.");
    }

    let number_of_bytes = self.cursor.value.length_in_bytes;
    let _ = self.read_exact(number_of_bytes)?;
    Ok(Some(&self.buffer))
  }

  pub fn annotations(&self) -> &[usize] {
    &self.cursor.value.annotations
  }

  pub fn field_id(&self) -> Option<usize> {
    self.cursor.value.field_id
  }

  pub fn new(data_source: &mut R) -> Result<BinaryIonCursor<R>, IonError> {
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

  pub fn next(&mut self) -> Result<Option<IonType>, IonError> {
    let _ = self.skip_current_value()?;

    if let Some(ref parent) = self.cursor.value.parent {
      // If the cursor is nested inside a parent object, don't attempt to read beyond the end of
      // the parent. Users can call '.step_out()' to progress beyond the container.
      if self.cursor.bytes_read >= parent.last_byte {
        debug!("We've run out of values in this parent.");
        return Ok(None);
      }
    }

    // If we're in a struct, read the field id that must precede each value.
    self.cursor.value.field_id = match self.cursor.is_in_struct {
      true => {
        Some(self.read_field_id()?)
      },
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

  fn read_var_uint(&mut self) -> Result<VarUInt, IonError> {
    let var_uint = VarUInt::read_var_uint(&mut self.data_source)?;
    self.cursor.bytes_read += var_uint.size_in_bytes();
    Ok(var_uint)
  }

  fn read_uint(&mut self) -> Result<UInt, IonError> {
    let number_of_bytes = self.cursor.value.length_in_bytes;
    let uint = UInt::read_uint(&mut self.data_source, number_of_bytes)?;
    self.cursor.bytes_read += uint.size_in_bytes();
    Ok(uint)
  }

  fn read_exact(&mut self, number_of_bytes: usize) -> Result<(), IonError> {
    self.buffer.resize(number_of_bytes, 0); // Will grow the buffer if needed.
    let _ = self.data_source.read_exact(&mut self.buffer)?;
    self.cursor.bytes_read += number_of_bytes;
    Ok(())
  }

  fn process_header_by_type_code(&mut self, header: &IonValueHeader) -> Result<(), IonError> {
    let length_code = header.length_code;
    let ion_type_code = header.ion_type_code;

    self.cursor.value.ion_type = IonType::from(header.ion_type_code)?;
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

    debug!("Inferred length in bytes: {}", length);
    self.cursor.value.length_in_bytes = length;
    self.cursor.value.last_byte = self.cursor.bytes_read + length;
    debug!("Cursor has read {} bytes. Last byte for this {:?}: {}",
             self.cursor.bytes_read,
             self.cursor.value.ion_type,
             self.cursor.value.last_byte
    );
    Ok(())
  }

  fn read_standard_length(&mut self) -> Result<usize, IonError> {
    let length = match self.cursor.value.header.length_code {
      LENGTH_CODE_NULL => 0,
      LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_float_length(&mut self) -> Result<usize, IonError> {
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

  fn read_struct_length(&mut self) -> Result<usize, IonError> {
    let length = match self.cursor.value.header.length_code {
      LENGTH_CODE_NULL => 0,
      1 | LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
      magnitude => magnitude as usize
    };

    Ok(length)
  }

  fn read_next_value_header(&mut self) -> Result<Option<IonValueHeader>, IonError> {
    let next_byte: u8 = match self.next_byte() {
      Ok(Some(byte)) => byte, // This is the one-byte header of the next value.
      Ok(None) => return Ok(None), // There's no more data to read.
      Err(error) => return Err(error) // Something went wrong while reading the next byte.
    };

    debug!("Next byte: {} ({:X})", next_byte, next_byte);
    let (type_code, length_code) = nibbles_from_byte(next_byte);
    let ion_type_code = IonTypeCode::from(type_code)?;
    let header = IonValueHeader {
      ion_type_code,
      length_code
    };
    debug!("Next header at byte {}: {:?}", self.cursor.bytes_read, &header);
    Ok(Some(header))
  }

  fn next_byte(&mut self) -> Result<Option<u8>, IonError> {
    self.cursor.bytes_read += 1;
    match self.data_source.bytes().next() {
      None => Ok(None),
      Some(Ok(byte)) => Ok(Some(byte)),
      Some(Err(error)) => io_error(error.description())
    }
  }

  fn skip_bytes(&mut self, number_of_bytes: usize) -> Result<(), IonError> {
    use std::io;
    debug!("Before seek, bytes_read: {}, number to skip: {}", self.cursor.bytes_read, number_of_bytes);

    let _bytes_copied = io::copy(
      &mut self.data_source.by_ref().take(number_of_bytes as u64),
      &mut io::sink()
    )?;

    self.cursor.bytes_read += number_of_bytes;
    debug!("After seek, bytes_read: {}", self.cursor.bytes_read);
    Ok(())
  }

  fn skip_current_value(&mut self) -> Result<(), IonError> {
    if self.cursor.index_at_depth == 0 {
      debug!("Haven't read the first value yet. Not skipping.");
      Ok(())
    } else {
      let bytes_to_skip = self.cursor.value.last_byte - self.cursor.bytes_read;
      debug!("Moving to the next value by skipping {} bytes", bytes_to_skip);
      self.skip_bytes(bytes_to_skip)
    }
  }

  fn read_field_id(&mut self) -> Result<usize, IonError> {
    debug!("Reading the field_id");
    let var_uint = self.read_var_uint()?;
    let field_id = var_uint.value();
    debug!("Found field_id symbol {}", field_id);
    Ok(field_id)
  }

  fn read_annotations(&mut self) -> Result<(), IonError> {
    debug!("Reading annotations.");
    let annotations_and_value_length = self.read_standard_length()?;
    debug!("Annotations + value length: {}", annotations_and_value_length);
    let annotations_length = self.read_var_uint()?.value();
    debug!("Annotations length: {}", annotations_length);
    let mut bytes_read: usize = 0;
    while bytes_read < annotations_length {
      let var_uint = self.read_var_uint()?;
      bytes_read += var_uint.size_in_bytes();
      let annotation_symbol_id = var_uint.value();
      self.cursor.value.annotations.push(annotation_symbol_id);
    }
    Ok(())
  }

  pub fn step_in(&mut self) -> Result<(), IonError> {
    use self::IonType::*;
    use std::mem::swap;
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
    debug!("Stepping into parent: {:?}", parent);
    self.cursor.value.parent = Some(Box::new(parent));
    self.cursor.depth += 1;
    self.cursor.index_at_depth = 0;
    Ok(())
  }

  fn step_out(&mut self) -> Result<(), IonError> {
    use std::mem::swap;
    let bytes_to_skip;
    let mut parent_value;
    { // parent scope
      let parent = match self.cursor.value.parent {
        Some(ref parent) => parent,
        None => panic!("You cannot step out of the root level.")
      };
      debug!("Currently at byte: {}", self.cursor.bytes_read);
      debug!("Stepping out of parent: {:?}", parent);
      bytes_to_skip = parent.last_byte - self.cursor.bytes_read;
      debug!("Bytes to skip: {}", bytes_to_skip);
      parent_value = (**parent).clone();
    }
    // Revert the cursor's current value to be the parent we stepped into.
    // After some bookkeeping, we'll skip enough bytes to move to the end of the parent.
    swap(&mut self.cursor.value, &mut parent_value);
    if let Some(ref parent) = self.cursor.value.parent {
      self.cursor.is_in_struct = parent.ion_type == IonType::Struct;
      debug!("Are we in a Struct? {}", self.cursor.is_in_struct);
    } else {
      self.cursor.is_in_struct = false;
    }
    self.cursor.index_at_depth = self.cursor.value.index_at_depth;
    self.cursor.depth -= 1;
    debug!("Stepping out by skipping {} bytes.", bytes_to_skip);
    self.skip_bytes(bytes_to_skip)?;
    debug!("After stepping out, bytes_read is: {}", self.cursor.bytes_read);
    Ok(())
  }
}