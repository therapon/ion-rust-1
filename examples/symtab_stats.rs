extern crate amzn_ion;
extern crate flate2;

use amzn_ion::binary::ion_cursor::{IonDataSource};
use amzn_ion::result::IonResult;
use amzn_ion::types::*;

use std::fs::File;
use std::collections::HashMap;
use amzn_ion::binary::BinaryIonReader;
use flate2::bufread::GzDecoder;
use std::rc::Rc;

fn collect_stats(path: &str) -> IonResult<()> {

  if path.ends_with(".gz") {
    let file = File::open(path).expect("Unable to open file");
    let buf_reader = std::io::BufReader::with_capacity(64 * 1024, file);
    let gz_decoder = GzDecoder::new(buf_reader);
    let gz_buf_reader = std::io::BufReader::with_capacity(64 * 1024, gz_decoder);
    let mut reader = BinaryIonReader::new(gz_buf_reader)?;
    stats(&mut reader)
  } else {
    let mut file = File::open(path).expect("Unable to open file");
    let buf_reader = std::io::BufReader::with_capacity(64 * 1024, file);
    let mut reader = BinaryIonReader::new(buf_reader)?;
    stats(&mut reader)
  }
}

fn stats<R: IonDataSource>(reader: &mut BinaryIonReader<R>) -> IonResult<()> {
  let mut string_counts: HashMap<String, usize> = HashMap::new();
  let mut symbol_counts: HashMap<IonSymbol, usize> = HashMap::new();
  loop {
    if let Some(ion_type) = reader.next()? {
      if reader.is_null() {
        continue;
      }
      match ion_type {
        IonType::Struct | IonType::List | IonType::SExpression => {
          let _ = reader.step_in()?;
//          let _ = skim_values(cursor, depth + 1)?;
//          let _ = cursor.step_out()?;
        },
        IonType::String => {
          let text = reader.read_string()?.unwrap();
          *(string_counts.entry(text).or_insert(0)) += 1;
        },
        IonType::Symbol => {
          let symbol = reader.read_symbol()?.unwrap();
          *(symbol_counts.entry(symbol).or_insert(0)) += 1;
        },
        _ => {}
//        Integer => {
//          let _int = cursor.read_i64()?.unwrap();
//        },
//        Float => {
//          let _float = cursor.read_f64()?.unwrap();
//        },
//        Decimal => {
//          let _decimal = cursor.read_decimal()?.unwrap();
//        }
//        Timestamp => {
//          let _timestamp = cursor.read_timestamp()?.unwrap();
//        }
//        Boolean => {
//          let _boolean = cursor.read_bool()?.unwrap();
//        },
//        Blob => {
//          let _blob = cursor.blob_ref_map(|_b| ())?.unwrap();
//        },
//        Clob => {
//          let _clob = cursor.clob_ref_map(|_c| ())?.unwrap();
//        }
//        Null => {}
      }
    } else if reader.depth() > 0 { // it was `None`
      reader.step_out()?;
    } else {
      break;
    }
  }

  // Hold the STDOUT lock so printing is faster
  let stdout = std::io::stdout();
  let _stdout_lock = stdout.lock();

  let top_n = 200usize;
  println!("----- Top {} Strings out of {} -----", top_n, string_counts.len());
  let mut string_count_entries: Vec<(&String, &usize)> = string_counts.iter().collect();
  string_count_entries.sort_by(|e1, e2| e1.1.cmp(e2.1).reverse());

  for &(ref text, ref count) in string_count_entries.iter().take(top_n) {
    println!("{} -> {}", count, text);
  }

  println!();
  println!("----- Top {} Symbols out of {} -----", top_n, symbol_counts.len());
  let mut symbol_count_entries: Vec<(&IonSymbol, &usize)> = symbol_counts.iter().collect();
  symbol_count_entries.sort_by(|e1, e2| e1.1.cmp(e2.1).reverse());

  for &(ref symbol, ref count) in symbol_count_entries.iter().take(top_n) {
    println!("{} -> {} (${})", count, symbol.text().unwrap_or("???"), u64::from(symbol.id()));
  }

  println!();
  println!("----- Bottom {} Symbols out of {} -----", top_n, symbol_counts.len());
  for &(ref symbol, ref count) in symbol_count_entries.iter().rev().take(5000) {
    println!("{} -> {} (${})", count, symbol.text().unwrap_or("???"), u64::from(symbol.id()));
  }

  println!();
  println!("----- Symbol Usage Distribution -----");
  // Symbol usage distribution
  let mut current_count: usize = 0;
  let mut num_symbols_with_current_count: usize = 0;
  for (ref _symbol, ref count) in symbol_count_entries.iter().rev() {
    let count = **count;
    if count == current_count {
      num_symbols_with_current_count += 1;
    } else {
      if current_count > 0 {
        println!("{} symbols had {} usages.", num_symbols_with_current_count, current_count);
        num_symbols_with_current_count = 1;
      }
      current_count = count;
    }
  }

  let one_off_symbols = symbol_count_entries
    .iter()
    .rev()
    .take_while(|e| *e.1 == 1)
    .count();
  println!();
  println!("{} symbols were only used once.", one_off_symbols);
  println!("Symbol table had {} symbols in it.", reader.symbol_table().len());

  println!();
  println!("=== SYMBOL TABLE ===");

  for (sid, text) in reader.symbol_table().iter() {
    //TODO: IonSymbol requires terrible machinations to instantiate
    //symbol_counts.get(&IonSymbol::new(0, Some(Rc::new(text.unwrap().to_owned())))).unwrap_or(0)
    println!("#{} -> {}", sid, text.unwrap_or("None"));
  }

  Ok(())
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let default_file = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  let path = args.get(1).map(|s| s.as_ref()).unwrap_or(default_file);

  println!("Analyzing file {}", path);

//  let path = "EC2_EFFICIENCY.10n";
  match collect_stats(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}
