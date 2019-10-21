extern crate amzn_ion;

use amzn_ion::binary::ion_cursor::{IonDataSource};
use amzn_ion::binary::BinaryIonReader;
use amzn_ion::result::IonResult;

use std::fs::File;

//extern crate jemallocator;

//#[global_allocator]
//static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn materialize_values_in_file(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let buf_reader = std::io::BufReader::with_capacity(64 * 1024, file);
  let mut reader = BinaryIonReader::new(buf_reader)?;

  materialize_values(&mut reader)
}

fn materialize_values<R: IonDataSource>(reader: &mut BinaryIonReader<R>) -> IonResult<()> {
  let mut count = 0;
  for ion_value in reader.ion_dom_values() {
//  for ion_value in reader.ion_dom_values().take(5) {
//    println!("{:#?}", ion_value?);
    count += 1;
  }

  println!("Materialized {} top-level values", count);
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match materialize_values_in_file(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}