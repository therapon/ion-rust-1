extern crate amzn_ion;

use std::fs::File;
use std::io::BufReader;
use std::io;
use amzn_ion::binary::ion_cursor::BinaryIonCursor;
use amzn_ion::result::IonResult;
use amzn_ion::binary::ion_cursor::CursorState;
use amzn_ion::types::ion_type::IonType;

fn test_checkpoint_and_restore() {
//    let path = "/Users/zslayton/local_ion_data/ion_data2/annotated_values.10n";
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match checkpoint_and_restore(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}

fn checkpoint_and_restore(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let mut reader = BufReader::with_capacity(512 * 1_024, file);
  let mut cursor = BinaryIonCursor::new(&mut reader)?;
  // Skip a few values
  let _ = cursor.next()?;
  let _ = cursor.next()?;


  // Make a checkpoint
  let checkpoint = cursor.checkpoint();
  use std::mem::size_of;
  println!("Size of CursorState: {} bytes", size_of::<CursorState>());
  // Read an event
  let ion_type = cursor.next()?.expect("Not enough values in the file.");
  assert_eq!(ion_type, IonType::Struct);
  cursor.step_in()?;
  let ion_type = cursor.next()?.expect("Missing timestamp.");
  assert_eq!(ion_type, IonType::Integer);
  let timestamp1 = cursor.integer_value()?.expect("Missing integer.");
  println!("Timestamp 1: {:?}", timestamp1);

  // Rewind to the checkpoint
  let _ = cursor.restore(checkpoint)?;

  // Read the same event
  let ion_type = cursor.next()?.expect("Not enough values in the file 2.");
  assert_eq!(ion_type, IonType::Struct);
  cursor.step_in()?;
  let ion_type = cursor.next()?.expect("Missing timestamp 2.");
  assert_eq!(ion_type, IonType::Integer);
  let timestamp2 = cursor.integer_value()?.expect("Missing integer 2.");
  println!("Timestamp 2: {:?}", timestamp2);

  assert_eq!(timestamp1, timestamp2);

  Ok(())
}

fn main() {
  test_checkpoint_and_restore();
}