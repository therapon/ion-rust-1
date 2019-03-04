extern crate crossbeam;
extern crate ion_rust;
extern crate madvise;
extern crate memmap;

use std::fs::File;
use std::io;
use std::str::FromStr;

use madvise::{AccessPattern, AdviseMemory};
use memmap::{MmapOptions};

use ion_rust::binary::BinaryIonReader;
use ion_rust::binary::ion_cursor::IonDataSource;
use ion_rust::result::IonResult;
use ion_rust::types::IonType;

// Given a file path, mmap it and create `num_threads` parallel readers
fn parallel_skim_file(path: &str, num_threads: usize) -> IonResult<()> {

  let file = File::open(path).expect("Unable to open file");
  let mmap = unsafe { MmapOptions::new().map(&file)? };
  &mmap[..]
    .advise_memory_access(AccessPattern::Sequential)
    .expect("Advisory failed");

  let ion_data = &mmap[..];

  crossbeam::scope(|scope| {
    for thread_number in 0..num_threads {
      scope.spawn(move |_| {
        let io_cursor = io::Cursor::new(ion_data);
        let mut reader = BinaryIonReader::new(io_cursor).expect("Not valid Ion.");
        for _ in 0..thread_number { // Skip ahead by `thread_number` values.
          reader.next().expect("Thread failed to skip ahead at start.");
        }
        skim_every_n_values(&mut reader, num_threads).expect("Failure during skim.");
      });
    }
  }).unwrap();

  Ok(())
}

// Perform a non-DOM read of all leaf values inside the current top-level value, then skip `n-1`
// top-level values and repeat. Used to allow `n` threads to cooperatively read the same data source.
fn skim_every_n_values<R: IonDataSource>(reader: &mut BinaryIonReader<R>, n: usize) -> IonResult<()> {
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
        }
        String => {
          let _text = reader.string_ref_map(|_s| ())?.unwrap();
        }
        Symbol => {
          let _symbol_id = reader.read_symbol_id()?.unwrap();
        }
        Integer => {
          let _int = reader.read_i64()?.unwrap();
        }
        Float => {
          let _float = reader.read_f64()?.unwrap();
        }
        Decimal => {
          let _decimal = reader.read_decimal()?.unwrap();
        }
        Timestamp => {
          //FIXME timestamp reading has a bug
//          let _timestamp = cursor.read_timestamp()?.unwrap();
        }
        Boolean => {
          let _boolean = reader.read_bool()?.unwrap();
        }
        Blob => {
          let _blob = reader.blob_ref_map(|_b| ())?.unwrap();
        }
        Clob => {
          let _clob = reader.clob_ref_map(|_c| ())?.unwrap();
        }
        Null => {}
      }
    } else if reader.depth() > 0 { // it was `None`
      reader.step_out()?;
      if reader.depth() == 0 {
        for _ in 1..n {
          assert_eq!(0, reader.depth());
          if let Ok(None) = reader.next() {
            break;
          }
        }
      }
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
  let path = args
    .get(1)
    .map(|s| s.as_ref())
    .unwrap_or(default_file);
  let num_threads = args
    .get(2)
    .map(|s| s.as_ref())
    .map(|s| usize::from_str(s).expect("Invalid threadcount."))
    .unwrap_or(4);

  match parallel_skim_file(path, num_threads) {
    Ok(_) => {}
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}