extern crate amzn_ion;

use amzn_ion::binary::ion_cursor::{IonDataSource, BinaryIonCursor};
use amzn_ion::result::IonResult;
use amzn_ion::types::IonType;

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io;
use std::io::BufRead;

fn old_byte_iter_test(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let mut reader = BufReader::with_capacity(8*1024, file);
  let mut sum: u64 = 0;
  let mut count: u64 = 0;

  for byte in reader.bytes() {
    let byte = byte?;
    sum += byte as u64;
    count += 1;
  }

  println!("Sum  : {}", sum);
  println!("Count: {}", count);
  Ok(())
}

fn new_byte_iter_test(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let mut reader = BufReader::with_capacity(8*1024, file);
  let mut sum: u64 = 0;
  let mut count: u64 = 0;

  let mut number_of_buffered_bytes;
  loop {
    {
      let buffer = reader.fill_buf()?;
      number_of_buffered_bytes = buffer.len();
      if number_of_buffered_bytes == 0 {
        break;
      }
      for byte in buffer {
        sum += *byte as u64;
        count += 1;
      }
    }
    reader.consume(number_of_buffered_bytes);
  }

  println!("Sum  : {}", sum);
  println!("Count: {}", count);
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
//  match old_byte_iter_test(path) {
//    Ok(_) => {},
//    Err(error) => panic!("Failed to read the file: {:?}", error)
//  }
  match new_byte_iter_test(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}