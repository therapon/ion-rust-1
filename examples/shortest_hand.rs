extern crate amzn_ion;

use amzn_ion::binary::ion_cursor::BinaryIonCursor;
use amzn_ion::result::{IonResult, decoding_error};
use amzn_ion::types::ion_type::IonType;

use std::fs::File;
use std::io::BufReader;
use std::collections::HashMap;
use std::io::Read;
use std::hash::Hash;
use std::hash::Hasher;
use std::collections::HashSet;
use amzn_ion::types::ion_symbol::IonSymbolId;

const SYSTEM_SYMBOLS: &[&str] = &[
  "$0", // Symbol Zero
  "$ion",
  "$ion_1_0",
  "$ion_symbol_table",
  "name",
  "version",
  "imports",
  "symbols",
  "max_id",
  "$ion_shared_symbol_table"
];

#[derive(Debug)]
struct SymbolTable {
  symbols: Vec<String>,
}

impl SymbolTable {
  pub fn new() -> SymbolTable {
    SymbolTable {
      symbols: SYSTEM_SYMBOLS.iter().map(|s| s.to_string()).collect()
    }
  }

  pub fn intern(&mut self, text: String) -> usize {
    self.symbols.push(text);
    self.symbols.len()
  }

  pub fn resolve<I>(&self, index: I) -> Option<&str> where I: Into<IonSymbolId> {
    let index: usize = index.into().into();
    if index >= self.symbols.len() {
      return None;
    }
    Some(&self.symbols[index])
  }
}

struct BinaryIonReader<'cursor, R: Read + 'cursor> {
  pub symbols: SymbolTable,
  pub cursor: BinaryIonCursor<'cursor, R>
}

impl <'cursor, R: Read + 'cursor> BinaryIonReader<'cursor, R> {
  fn read_text(&mut self) -> IonResult<String> {
    let ion_type = self.cursor.ion_type();
    match ion_type {
      IonType::String => Ok(self.cursor.string_ref_value()?.unwrap().to_string()),
      IonType::Symbol => {
        let symbol_id = self.cursor.symbol_id_value()?.unwrap();
        //println!("Resolving: {:?} in {:?}, {} symbols", symbol_id, self.symbols, self.symbols.symbols.len());
        Ok(self.symbols.resolve(symbol_id).expect("Unknown symbol!").to_string())
      },
      _ => panic!("Tried to get text from a {:?}", ion_type)
    }
  }
}

#[derive(Debug)]
struct LogEvent {
  timestamp: u64,
  thread_id: u64,
  thread_name: String,
  logger_name: String,
  log_level: String,
  format: String,
  parameters: Vec<u32>,
  thread_context: HashMap<String, String>
}

impl PartialEq for LogEvent {
  fn eq(&self, other: &LogEvent) -> bool {
    self.format == other.format
    && self.logger_name == other.logger_name
    && self.log_level == other.log_level
  }
}

impl Eq for LogEvent {}

impl Hash for LogEvent {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.format.hash(state);
    self.logger_name.hash(state);
    self.log_level.hash(state);
  }
}

fn read_log_event<T: Read>(reader: &mut BinaryIonReader<T>) -> IonResult<LogEvent> {
  loop {
    if let Some(IonType::Struct) = reader.cursor.next()? {
      if reader.cursor.value_is_symbol_table() {
        let _ = read_symbol_table(reader)?;
        continue;
      }

      reader.cursor.step_in()?;

      let _ion_type = reader.cursor.next()?;
      //println!("Timestamp is a {:?}", ion_type);
      let timestamp = reader.cursor.integer_value()?.unwrap().into();

      let _ion_type = reader.cursor.next()?;
//      println!("Thread ID is a {:?}", ion_type);
      let thread_id = reader.cursor.integer_value()?.unwrap().into();

      let _ion_type = reader.cursor.next()?;
//      println!("Thread name is a {:?}", ion_type);
      let thread_name = reader.read_text()?;//cursor.symbol_value()?.unwrap();

      let _ion_type = reader.cursor.next()?;
//      println!("Logger name is a {:?}", ion_type);
      let logger_name = reader.read_text()?;//cursor.symbol_value()?.unwrap();

      let _ion_type = reader.cursor.next()?;
//      println!("Log level is a {:?}", ion_type);
      let log_level = reader.read_text()?;//cursor.symbol_value()?.unwrap();

      let _ion_type = reader.cursor.next()?;
//      println!("Format is a {:?}", ion_type);
      let format = reader.read_text()?;//cursor.symbol_value()?.unwrap();

      let _ = reader.cursor.step_out()?;

      return Ok(LogEvent {
        timestamp,
        thread_id,
        thread_name,
        logger_name,
        log_level,
        format,
        parameters: Vec::new(),
        thread_context: HashMap::new(),
      });
    } else {
      return decoding_error("Tried to read a value that wasn't a Struct.");
    }
  }
}

fn read_symbol_table<T: Read>(reader: &mut BinaryIonReader<T>) -> IonResult<()> {
  if !reader.cursor.value_is_symbol_table() {
    panic!("Tried to read a struct that wasn't a symbol table. It had no annotations.");
  }

//  println!("Reading a symbool table.");
  let _ = reader.cursor.step_in();
  read_symbol_table_field(reader)?;
  read_symbol_table_field(reader)?;
  let _ = reader.cursor.step_out();
  Ok(())
}

fn read_symbol_table_field<T: Read>(reader: &mut BinaryIonReader<T>) -> IonResult<()> {
  let _ion_type = reader.cursor.next()?;
  let field_id: Option<usize> = reader.cursor.field_id().map(Into::into);
//  println!("Reading symbol table field: {:?}", field_id);
  match field_id {
    Some(6 /*imports*/) => Ok(()), // Do nothing
    Some(7 /*symbols*/) => read_symbol_list(reader),
    Some(symbol_id) => panic!("Unrecognized field: {}", symbol_id),
    None => Ok(())
  }
}

fn read_symbol_list<T: Read>(reader: &mut BinaryIonReader<T>) -> IonResult<()> {
  let ion_type = reader.cursor.ion_type();
  if ion_type != IonType::List {
    panic!("Expected the symbol list to be an IonType::List, but it was a {:?}", ion_type);
  }
//  println!("Reading a symbol list.");
  let _ = reader.cursor.step_in()?;
  while let Some(IonType::String) = reader.cursor.next()? {
    let text = reader.cursor.string_ref_value()?.unwrap().to_string();
//    println!("Interning: {}", text);
    reader.symbols.intern(text);
  }
  let _ = reader.cursor.step_out()?;
  Ok(())
}

fn unique_log_statements(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");

  let mut buf_reader = BufReader::with_capacity(32 * 1_024, file);
  let cursor = BinaryIonCursor::new(&mut buf_reader)?;

  let mut ion_reader = BinaryIonReader {
    symbols: SymbolTable::new(),
    cursor
  };

  let mut log_statements = HashSet::new();

  let mut count = 0;
  while let Ok(event) = read_log_event(&mut ion_reader) {
//    println!("{:?}", event);
    log_statements.insert(event);
    count += 1;
  }

//  for (index, log_statement) in log_statements.iter().enumerate() {
//    println!("{}. {:?}", index + 1, log_statement);
//  }
  println!("Read {} events", count);
  println!("Found {} unique events", log_statements.len());
  println!("Interned {} symbols while reading.", ion_reader.symbols.symbols.len());
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match unique_log_statements(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}