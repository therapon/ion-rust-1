extern crate amzn_ion;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::BufRead;
use std::io::Read;

use amzn_ion::binary::BinaryIonReader;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;
use amzn_ion::result::{decoding_error, IonResult};
use amzn_ion::types::*;
use amzn_ion::types::IonType;

#[derive(Debug)]
struct LogEvent {
  pub timestamp: u64,
  pub thread_id: u64,
  pub thread_name: String,
  pub logger_name: String,
  pub log_level: String,
  pub format: String,
  pub parameters: Vec<IonDomValue>,
  pub thread_context: HashMap<String, String>
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

fn read_log_event<T: BufRead>(reader: &mut BinaryIonReader<T>) -> IonResult<LogEvent> {
  if let Some(IonType::Struct) = reader.next()? {
    reader.step_in()?;

    let _ion_type = reader.next()?;
    //println!("Timestamp is a {:?}", ion_type);
    let timestamp = reader.read_i64()?.unwrap() as u64;

    let _ion_type = reader.next()?;
//      println!("Thread ID is a {:?}", ion_type);
    let thread_id = reader.read_i64()?.unwrap() as u64;

    let _ion_type = reader.next()?;
//      println!("Thread name is a {:?}", ion_type);
    let thread_name = reader.read_text()?;//cursor.symbol_value()?.unwrap();

    let _ion_type = reader.next()?;
//      println!("Logger name is a {:?}", ion_type);
    let logger_name = reader.read_text()?;//cursor.symbol_value()?.unwrap();

    let _ion_type = reader.next()?;
//      println!("Log level is a {:?}", ion_type);
    let log_level = reader.read_text()?;//cursor.symbol_value()?.unwrap();

    let _ion_type = reader.next()?;
//      println!("Format is a {:?}", ion_type);
    let format = reader.read_text()?;//cursor.symbol_value()?.unwrap();

    //TODO: IonValue, not IonDomValue
    let mut parameters: Vec<IonDomValue> = vec![];
    reader.next()?;
    reader.step_in()?;
    while let Some(ion_type) = reader.next()? {
      parameters.push(reader.ion_dom_value()?);
    }
    reader.step_out()?;

    reader.next()?; // SKIP OVER THROWABLE FOR NOW

    let mut thread_context = HashMap::new();
    reader.next()?;
    reader.step_in()?; // We're inside a struct.
    while let Some(_ion_type) = reader.next()? {
      let field_name = reader.field()?.unwrap().text().unwrap().to_owned();
      let value = reader.read_text()?;
      thread_context.insert(field_name, value);
    }
    reader.step_out()?;

    //TODO: parameters, thread context, throwable

    let _ = reader.step_out()?;

    return Ok(LogEvent {
      timestamp,
      thread_id,
      thread_name,
      logger_name,
      log_level,
      format,
      parameters,
      thread_context,
    });
  } else {
    return decoding_error("Tried to read a value that wasn't a Struct.");
  }
}

fn unique_log_statements(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let buf_reader = std::io::BufReader::with_capacity(64 * 1024, file);
  let mut ion_reader = BinaryIonReader::new(buf_reader)?;

  let mut log_statements: HashSet<LogEvent> = HashSet::new();

  let dsn = "G000MW067184001E";//"G0B0VV028247002D";

  let mut count = 0;
  while let Ok(event) = read_log_event(&mut ion_reader) {
    count += 1;
    println!("{}. ts {:?} // thread {:?} // logger {:?} // {:?}, {:?}", count, event.timestamp, event.thread_name, event.logger_name, event.format, event.parameters);
//    if event.format == dsn
//      || event.thread_context.values().any(|c| c == dsn)
//      || event.parameters.iter().any(|p| {
//      match &p.value() {
////        IonValue::String(ion_string) if ion_string.len() == 16 => true,
//        IonValue::String(ion_string) if ion_string.as_ref() == dsn => true,
//        IonValue::Symbol(ion_symbol) if ion_symbol.text() == Some(dsn) => true,
//        _ => false
//      }
//    }) {
//      println!("{}. {:?}, {:?}", count, event.format, event.parameters);
//      count +=1;
//    }



//    log_statements.insert(event);


//    if count > 100 {
//      return Ok(());
//    }
  }

//  for (index, log_statement) in log_statements.iter().enumerate() {
//    println!("{}. {:?}", index + 1, log_statement);
//  }

  println!("Read {} events", count);
  println!("Found {} unique events", log_statements.len());
  println!("Interned {} symbols while reading.", ion_reader.symbol_table().len());
  Ok(())
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let default_file = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  let path = args.get(1).map(|s| s.as_ref()).unwrap_or(default_file);

  match unique_log_statements(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}