extern crate amzn_ion;
extern crate memmap;
extern crate madvise;

use std::io;
use std::io::BufReader;
use std::fs::File;
use amzn_ion::result::IonResult;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;
use memmap::{Mmap, MmapOptions};
use madvise::{AccessPattern, AdviseMemory};

fn count_top_level_values(path: &str) -> IonResult<()> {
  println!("Path: :{}", path);
  let file = File::open(path).expect("Unable to open file");

//  let mut reader = BufReader::with_capacity(128 * 1_024, file);
//  let mut cursor = BinaryIonCursor::new(&mut reader)?;

  let mmap = unsafe { MmapOptions::new().map(&file)? };
  &mmap[..].advise_memory_access(AccessPattern::Sequential).expect("Advisory failed");
  let io_cursor = io::Cursor::new(&mmap[..]);
  let mut cursor = BinaryIonCursor::new(io_cursor)?;


  let mut count = 0;
  while let Some(_) = cursor.next()? {
    count += 1;
  }
  println!("Skip-scanned {} values", count);
  Ok(())
}

fn main() {
  let args: Vec<String> = std::env::args().collect();
  let default_file = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  let path = args.get(1).map(|s| s.as_ref()).unwrap_or(default_file);
  match count_top_level_values(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}