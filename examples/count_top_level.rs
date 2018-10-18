extern crate amzn_ion;

use std::fs::File;
use std::io::BufReader;
use std::io;
use amzn_ion::result::IonResult;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;

fn skip_all_values(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");

  let mut reader = BufReader::with_capacity(32 * 1_024, file);
  let mut cursor = BinaryIonCursor::new(&mut reader)?;

  let mut count = 0;
  while let Some(_) = cursor.next()? {
    count += 1;
  }
  println!("Skip-scanned {} values", count);
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match skip_all_values(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}