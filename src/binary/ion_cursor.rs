use std::io::{Read, Seek};

use bytes::BigEndian;
use bytes::ByteOrder;

use crate::binary::header_byte::*;
use crate::binary::ion_type_code::IonTypeCode;
use crate::binary::uint::UInt;
use crate::binary::var_uint::VarUInt;

use crate::types::*;

use crate::result::{decoding_error, IonError, IonResult};

use crate::binary::var_int::VarInt;
use std::io::BufRead;

use crate::binary::int::Int;
use bigdecimal::BigDecimal;
use chrono::offset::FixedOffset;
use chrono::prelude::*;
use std::io;

const LENGTH_CODE_NULL: u8 = 15;
const LENGTH_CODE_VAR_UINT: u8 = 14;

const IVM_LENGTH: usize = 4;
const IVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

#[derive(Copy, Clone, Debug)]
pub(crate) struct IonValueHeader {
    // We precalculate all of the possible IonValueHeaders during cursor init.
    pub ion_type: Option<IonType>,
    pub ion_type_code: IonTypeCode,
    pub length_code: u8,
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
    annotations: Vec<IonSymbolId>,
    parent_index: Option<usize>,
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
    bytes_read: usize,     // How many bytes we've read from `data_source`
    depth: usize,          // How deeply nested the cursor is at the moment
    index_at_depth: usize, // The number of values (starting with 0) read at the current depth
    is_in_struct: bool,    // Whether this level of descent is within a struct
    value: CursorValue,    // Information about the value on which the cursor is currently sitting.
    parents: Vec<CursorValue>,
}

// A low-level reader that offers no validation or symbol management.
// It can only move and return the current value.
pub struct BinaryIonCursor<R>
where
    R: IonDataSource,
{
    data_source: R,  // Our source of binary Ion bytes. It may or may not be able to seek.
    buffer: Vec<u8>, // Used for individual data_source.read() calls independent of input buffering
    cursor: CursorState,
    // TODO: This should be a const living somewhere. Unfortunately,
    // `lazy_static` adds a LOT of accessor overhead.
    header_cache: Vec<IonResult<Option<IonValueHeader>>>,
}

//TODO Move other I/O functions here? Remove all I/O functions and just depend on BufRead?
pub trait IonDataSource: BufRead {
    fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()>;
}

// In general, when we need to skip over a value (or step out of a value) we can do so by simply
// consuming the next `N` bytes from the data source.
impl<T: BufRead> IonDataSource for T {
    default fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
        use std::io;
        let _bytes_copied = io::copy(
            &mut self.by_ref().take(number_of_bytes as u64),
            &mut io::sink(), // Effectively /dev/null
        )?;
        Ok(())
    }
}

// When reading from an in-memory byte array instead of from a stream, we can skip bytes by jumping
// to the desired index instead of scanning ahead by `N` bytes.
// We don't do the same for other data sources that offer seek()-style functionality because we
// don't know the cost. File system seeking can take ~1ms, making it much more expensive than
// small scans over bytes that are already buffered.
impl<T: BufRead + AsRef<[u8]>> IonDataSource for io::Cursor<T> {
    fn skip_bytes(&mut self, number_of_bytes: usize) -> IonResult<()> {
        self.set_position(self.position() + number_of_bytes as u64);
        Ok(())
    }
}

// If our data source allows us to seek backwards, we can provide a checkpoint-and-restore API that
// allows you to rewind the cursor to an earlier position.
impl<R> BinaryIonCursor<R>
where
    R: IonDataSource + Seek,
{
    pub fn checkpoint(&self) -> CursorState {
        self.cursor.clone()
    }

    pub fn restore(&mut self, mut saved_state: CursorState) -> IonResult<CursorState> {
        use std::io::SeekFrom;
        use std::mem;
        mem::swap(&mut self.cursor, &mut saved_state);
        let seeker = &mut self.data_source as &mut Seek;
        seeker.seek(SeekFrom::Start(self.cursor.bytes_read as u64))?;
        Ok(saved_state)
    }
}

impl<R> BinaryIonCursor<R>
where
    R: IonDataSource,
{
    pub fn is_null(&self) -> bool {
        self.cursor.value.is_null
    }

    pub fn ion_type(&self) -> IonType {
        self.cursor.value.ion_type
    }

    pub fn depth(&self) -> usize {
        self.cursor.depth
    }

    // TODO:
    // - Detect overflow and report an error
    // - Add an integer_size() method that indicates whether the current value will
    //   fit in an i32, i64, i128 or BigInteger. Offer corresponding read_* methods.
    pub fn read_i64(&mut self) -> IonResult<Option<i64>> {
        //    println!("type: {:?}", self.cursor.value.ion_type);
        //    println!("is_null: {:?}", self.cursor.value.is_null);
        if self.cursor.value.ion_type != IonType::Integer || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same integer value more than once.");
        }

        //    self.unchecked_read_i64().map(|r| Some(r))
        self.unchecked_read_i64()
    }

    // TODO: Experimental. In some circumstances, we already know that the current value
    // is an integer and hasn't been read yet. In those cases, we can bypass the safety checks
    // performed by read_i64(). Benchmarks have shown this to be a speedup, but not always.
    // The compiler can sometimes eliminate redundant checks.
    pub fn unchecked_read_i64(&mut self) -> IonResult<Option<i64>> {
        use self::IonTypeCode::*;

        let magnitude = self.read_value_as_uint()?.value();

        let value = match self.cursor.value.header.ion_type_code {
            PositiveInteger => magnitude as i64,
            NegativeInteger => -(magnitude as i64),
            _ => unreachable!("The Ion Type Code must be one of the above to reach this point."),
        };

        Ok(Some(value))
    }

    // TODO: Currently this just defers to read_f64 and then downcasts, losing precision.
    // We should offer a float_size() method to allow callers to call read_f32 or read_f64
    // as needed. Calling read_f32 on a 64-bit float should return an error.
    pub fn read_f32(&mut self) -> IonResult<Option<f32>> {
        match self.read_f64() {
            Ok(Some(value)) => Ok(Some(value as f32)), // Lossy if the value was 64 bits
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn read_f64(&mut self) -> IonResult<Option<f64>> {
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
                _ => {
                    return decoding_error(&format!(
                        "Encountered an illegal value for a Float length: {}",
                        number_of_bytes
                    ))
                }
            };
            Ok(Some(value))
        })
    }

    pub fn read_bool(&mut self) -> Result<Option<bool>, IonError> {
        if self.cursor.value.ion_type != IonType::Boolean || self.cursor.value.is_null {
            return Ok(None);
        }

        // No reading from the stream occurs -- the header contains all of the information we need.
        let representation = self.cursor.value.header.length_code;

        match representation {
            0 => Ok(Some(false)),
            1 => Ok(Some(true)),
            _ => decoding_error(&format!(
                "Found a boolean value with an illegal representation: {}",
                representation
            )),
        }
    }

    pub fn read_string(&mut self) -> IonResult<Option<String>> {
        self.string_ref_map(|s: &str| s.into())
    }

    pub fn string_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>>
    where
        F: FnOnce(&str) -> T,
    {
        use std::str;
        if self.cursor.value.ion_type != IonType::String || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same string value more than once.");
        }

        let length_in_bytes = self.cursor.value.length_in_bytes;

        self.parse_n_bytes(length_in_bytes, |buffer: &[u8]| {
            let string_ref = match str::from_utf8(buffer) {
                Ok(utf8_text) => utf8_text,
                Err(utf8_error) => {
                    return decoding_error(&format!(
                        "The requested string was not valid UTF-8: {:?}",
                        utf8_error
                    ))
                }
            };
            Ok(Some(f(string_ref)))
        })
    }

    pub fn read_symbol_id(&mut self) -> IonResult<Option<usize>> {
        if self.cursor.value.ion_type != IonType::Symbol || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same symbol ID value more than once.");
        }

        //TODO: This currently casts a u64 to a usize. Should we use u64 instead?
        let symbol_id = self.read_value_as_uint()?.value() as usize;
        Ok(Some(symbol_id))
    }

    pub fn read_blob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
        self.blob_ref_map(|b| b.into())
    }

    pub fn blob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>>
    where
        F: FnOnce(&[u8]) -> T,
    {
        if self.cursor.value.ion_type != IonType::Blob || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same blob value more than once.");
        }

        let number_of_bytes = self.cursor.value.length_in_bytes;
        self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| Ok(Some(f(buffer))))
    }

    pub fn read_clob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
        self.clob_ref_map(|c| c.into())
    }

    pub fn clob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>>
    where
        F: FnOnce(&[u8]) -> T,
    {
        if self.cursor.value.ion_type != IonType::Clob || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same clob value more than once.");
        }

        let number_of_bytes = self.cursor.value.length_in_bytes;
        self.parse_n_bytes(number_of_bytes, |buffer: &[u8]| Ok(Some(f(buffer))))
    }

    pub fn read_decimal(&mut self) -> IonResult<Option<BigDecimal>> {
        if self.cursor.value.ion_type != IonType::Decimal || self.cursor.value.is_null {
            return Ok(None);
        }

        if self.cursor.value.length_in_bytes > 0 && self.finished_reading_value() {
            panic!("You cannot read the same decimal value more than once.");
        }

        let exponent_var_int = self.read_var_int()?;
        let coefficient_size_in_bytes =
            self.cursor.value.length_in_bytes - exponent_var_int.size_in_bytes();

        let exponent = exponent_var_int.value() as i64;
        let coefficient = self.read_int(coefficient_size_in_bytes)?.value();

        // BigDecimal uses 'scale' rather than 'exponent' in its API, which is a count of the number of
        // decimal places. It's effectively `exponent * -1`.
        Ok(Some(BigDecimal::new(coefficient.into(), -exponent)))
    }

    fn finished_reading_value(&mut self) -> bool {
        self.cursor.bytes_read >= self.cursor.value.last_byte
    }

    pub fn read_timestamp(&mut self) -> IonResult<Option<DateTime<FixedOffset>>> {
        if self.cursor.value.ion_type != IonType::Timestamp || self.cursor.value.is_null {
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

            // TODO: Fractional seconds. Need to determine the least lossy way to turn our decimal
            // value into something the NaiveDate builder will take. Might be ok to limit supported
            // precision to nanoseconds.
        }
        let naive_datetime = NaiveDate::from_ymd(year as i32, month as u32, day as u32).and_hms(
            hour as u32,
            minute as u32,
            second as u32,
        );
        let offset = FixedOffset::west(offset_minutes as i32 * 60i32);
        let datetime = offset.from_utc_datetime(&naive_datetime);
        Ok(Some(datetime))
    }

    pub fn annotation_ids<'a>(&'a self) -> impl Iterator<Item = IonSymbolId> + 'a {
        self.cursor.value.annotations.iter().cloned()
    }

    pub fn field_id(&self) -> Option<IonSymbolId> {
        self.cursor.value.field_id
    }

    pub fn value_is_symbol_table(&self) -> bool {
        let symbol_id = match (self.ion_type(), self.annotation_ids().next()) {
            (IonType::Struct, Some(symbol_id)) => symbol_id,
            _ => return false,
        };
        symbol_id == IonSymbolId::from(3u64)
    }

    //TODO: This can just return Self if we defer reading the IVM.
    //We should do this anyway because reading the IVM should happen inside of next().
    //Any number of IVMs can appear in the input and we need to reset the symbol table when this
    //occurs.
    pub fn new(mut data_source: R) -> IonResult<Self> {
        let buffer = &mut [0u8; 4];
        let _ = data_source.read_exact(buffer)?;
        if *buffer != IVM {
            return decoding_error(&format!(
                "The data source must begin with an Ion Version Marker ({:?}). Found: ({:?})",
                IVM, buffer
            ));
        }

        Ok(BinaryIonCursor {
            data_source: data_source,
            buffer: vec![0; 1024],
            cursor: CursorState {
                bytes_read: IVM_LENGTH,
                depth: 0,
                index_at_depth: 0,
                is_in_struct: false,
                value: Default::default(),
                parents: Vec::new(),
            },
            header_cache: SLOW_HEADERS.clone(), // TODO: Bad. Make this static.
        })
    }

    pub fn next(&mut self) -> IonResult<Option<IonType>> {
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
            false => None,
        };

        // Pull the next byte from the data source and interpret it as a value header
        let mut header = match self.read_next_value_header()? {
            Some(header) => header,
            None => return Ok(None), // TODO: update ion_type() value to be None?
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
                None => return Ok(None),
            };
            self.cursor.value.header = header;
        }

        let _ = self.process_header_by_type_code(header)?;

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

        self.data_source.read_exact(buffer)?;
        self.cursor.bytes_read += number_of_bytes;
        Ok(())
    }

    fn parse_n_bytes<T, F>(&mut self, number_of_bytes: usize, processor: F) -> IonResult<T>
    where
        F: FnOnce(&[u8]) -> IonResult<T>,
    {
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

    fn process_header_by_type_code(&mut self, header: IonValueHeader) -> IonResult<()> {
        use self::IonTypeCode::*;

        // TODO: We end up unnecessarily storing `ion_type` in two places:
        // cursor.value.ion_type
        // cursor.value.header.ion_type
        // This is a waste of space and a waste of a memcpy. We should define self.ion_type() to read
        // from the header value and then call that everywhere we need to read the value.

        self.cursor.value.ion_type = header.ion_type.unwrap();
        self.cursor.value.header = header;
        self.cursor.value.is_null = header.length_code == LENGTH_CODE_NULL;

        // The spec defines special length code meanings for Float and Struct. All other non-zero length
        // types interpret the length code the same way.
        let length = match header.ion_type_code {
            Null | Boolean => 0,
            PositiveInteger | NegativeInteger | Decimal | Timestamp | String | Symbol | List
            | SExpression | Clob | Blob => self.read_standard_length()?,
            Float => self.read_float_length()?,
            Struct => self.read_struct_length()?,
            Annotation => return decoding_error("Found an annotation wrapping an annotation."),
            Reserved => return decoding_error("Found an Ion Value with a Reserved type code."),
        };

        self.cursor.value.length_in_bytes = length;
        self.cursor.value.last_byte = self.cursor.bytes_read + length;
        Ok(())
    }

    fn read_standard_length(&mut self) -> IonResult<usize> {
        let length = match self.cursor.value.header.length_code {
            LENGTH_CODE_NULL => 0,
            LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
            magnitude => magnitude as usize,
        };

        Ok(length)
    }

    fn read_float_length(&mut self) -> IonResult<usize> {
        let length = match self.cursor.value.header.length_code {
            0 => 0,
            4 => 4,
            8 => 8,
            LENGTH_CODE_NULL => 0,
            _ => {
                return decoding_error(format!(
                    "Found a Float value with an illegal length: {}",
                    self.cursor.value.header.length_code
                ))
            }
        };
        Ok(length)
    }

    fn read_struct_length(&mut self) -> IonResult<usize> {
        let length = match self.cursor.value.header.length_code {
            LENGTH_CODE_NULL => 0,
            1 | LENGTH_CODE_VAR_UINT => self.read_var_uint()?.value(),
            magnitude => magnitude as usize,
        };

        Ok(length)
    }

    fn read_next_value_header(&mut self) -> IonResult<Option<IonValueHeader>> {
        let next_byte: u8 = match self.next_byte() {
            Ok(Some(byte)) => byte,      // This is the one-byte header of the next value.
            Ok(None) => return Ok(None), // There's no more data to read.
            Err(error) => return Err(error), // Something went wrong while reading the next byte.
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

        // Some data sources like byte arrays or memory maps are able to seek forwards
        // efficiently, so we defer to the data source's implementation of skip_bytes().
        (&mut self.data_source as &mut IonDataSource).skip_bytes(number_of_bytes)?;
        self.cursor.bytes_read += number_of_bytes;
        Ok(())
    }

    fn skip_current_value(&mut self) -> IonResult<()> {
        // Calls to next() will call skip_current_value() to skip the rest of the value that's
        // currently being read. If we just started reading the stream or just called step_in(),
        // there is no current value, so we do nothing.
        if self.cursor.index_at_depth == 0 {
            return Ok(());
        }

        let bytes_to_skip = self.cursor.value.last_byte - self.cursor.bytes_read;
        self.skip_bytes(bytes_to_skip)
    }

    fn read_field_id(&mut self) -> IonResult<IonSymbolId> {
        let var_uint = self.read_var_uint()?;
        let field_id = var_uint.value().into();
        Ok(field_id)
    }

    fn read_annotations(&mut self) -> IonResult<()> {
        // Populates the cursor's `annotations` Vec with the symbol IDs of the annotations found
        // wrapping the next value.
        let _annotations_and_value_length = self.read_standard_length()?;
        let annotations_length = self.read_var_uint()?.value();
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
        self.cursor.is_in_struct = match self.ion_type() {
            Struct => true,
            List | SExpression => false,
            _ => panic!("You cannot step into a(n) {:?}", self.ion_type()),
        };

        self.cursor.value.parent_index = Some(self.cursor.parents.len());
        // Save our current state so we can return to this point later when we call step_out()
        self.cursor.parents.push(self.cursor.value.clone());
        self.cursor.depth += 1;
        self.cursor.index_at_depth = 0;
        Ok(())
    }

    pub fn step_out(&mut self) -> IonResult<()> {
        use std::mem::swap;
        let bytes_to_skip;

        {
            // Explicit scope to cause `parent` to be dropped

            // Remove the last parent from the parents vec
            let mut parent = self
                .cursor
                .parents
                .pop()
                .expect("You cannot step out of the root level.");

            bytes_to_skip = parent.last_byte - self.cursor.bytes_read;

            // Revert the cursor's current value to be the parent we stepped into.
            swap(&mut self.cursor.value, &mut parent);
        }
        // After some bookkeeping, we'll skip enough bytes to move to the end of the parent.

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
