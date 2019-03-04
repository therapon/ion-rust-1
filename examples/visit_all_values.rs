extern crate ion_rust;

extern crate flate2;
extern crate memmap;
extern crate madvise;

use ion_rust::binary::ion_cursor::{IonDataSource, BinaryIonCursor};
use ion_rust::result::IonResult;
use ion_rust::types::IonType;
use ion_rust::binary::BinaryIonReader;

use flate2::bufread::GzDecoder;
use memmap::{Mmap, MmapOptions};
use madvise::{AccessPattern, AdviseMemory};

use std::fs::File;


fn skim_file(path: &str) -> IonResult<()> {
  if path.ends_with(".gz") {
    let file = File::open(path).expect("Unable to open file");
    let buf_reader = std::io::BufReader::new(file);
    let gz_decoder = GzDecoder::new(buf_reader);
    let gz_buf_decoder = std::io::BufReader::new(gz_decoder);
    let mut cursor = BinaryIonReader::new(gz_buf_decoder)?;
    skim_values(&mut cursor)
  } else {
    let file = File::open(path).expect("Unable to open file");
    let buf_reader = std::io::BufReader::new(file);
    let mut cursor = BinaryIonReader::new(buf_reader)?;
    skim_values(&mut cursor)
  }
}

fn skim_values<R: IonDataSource>(reader: &mut BinaryIonReader<R>) -> IonResult<()> {
  use IonType::*;
  let mut count = 0;
  loop {
    if let Some(ion_type) = reader.next()? {
      count += 1;
      if reader.is_null() {
        continue;
      }
      match ion_type {
        Struct | List | SExpression => {
          let _ = reader.step_in()?;
//          let _ = skim_values(cursor, depth + 1)?;
//          let _ = cursor.step_out()?;
        },
        String => {
          let _text = reader.string_ref_map(|_s| ())?.unwrap();
        },
        Symbol => {
          let _symbol_id = reader.read_symbol_id()?.unwrap();
        },
        Integer => {
          let _int = reader.read_i64()?.unwrap();
        },
        Float => {
          let _float = reader.read_f64()?.unwrap();
        },
        Decimal => {
          let _decimal = reader.read_decimal()?.unwrap();
        }
        Timestamp => {
          //TODO
//          let _timestamp = cursor.read_timestamp()?.unwrap();
        }
        Boolean => {
          let _boolean = reader.read_bool()?.unwrap();
        },
        Blob => {
          let _blob = reader.blob_ref_map(|_b| ())?.unwrap();
        },
        Clob => {
          let _clob = reader.clob_ref_map(|_c| ())?.unwrap();
        }
        Null => {}
      }
    } else if reader.depth() > 0 { // it was `None`
      reader.step_out()?;
    } else {
      break;
    }
  }
  println!("Skimmed {} values", count);
  Ok(())
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let default_file = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  let path = args.get(1).map(|s| s.as_ref()).unwrap_or(default_file);

//  let path = "EC2_EFFICIENCY.10n";
  match skim_file(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}