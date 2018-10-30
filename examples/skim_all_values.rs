extern crate amzn_ion;

use amzn_ion::binary::ion_cursor::{IonDataSource, BinaryIonCursor};
use amzn_ion::result::IonResult;
use amzn_ion::types::IonType;

use std::fs::File;

fn skim_file(path: &str) -> IonResult<()> {
  let mut file = File::open(path).expect("Unable to open file");
  let mut cursor = BinaryIonCursor::new(&mut file)?;

  skim_values(&mut cursor)
}

fn skim_values<R: IonDataSource>(cursor: &mut BinaryIonCursor<R>) -> IonResult<()> {
  use IonType::*;
  let mut count = 0;
  loop {
    if let Some(ion_type) = cursor.next()? {
      count += 1;
      if cursor.is_null() {
        continue;
      }
      match ion_type {
        Struct | List | SExpression => {
          let _ = cursor.step_in()?;
//          let _ = skim_values(cursor, depth + 1)?;
//          let _ = cursor.step_out()?;
        },
        String => {
          let _text = cursor.string_ref_map(|_s| ())?.unwrap();
        },
        Symbol => {
          let _symbol_id = cursor.symbol_id_value()?.unwrap();
        },
        Integer => {
          let _int = cursor.integer_value()?.unwrap();
        },
        Float => {
          let _float = cursor.float_value()?.unwrap();
        },
        Decimal => {
          let _decimal = cursor.decimal_value()?.unwrap();
        }
        Timestamp => {
          let _timestamp = cursor.timestamp_value()?.unwrap();
        }
        Boolean => {
          let _boolean = cursor.boolean_value()?.unwrap();
        },
        Blob => {
          let _blob = cursor.blob_ref_map(|_b| ())?.unwrap();
        },
        Clob => {
          let _clob = cursor.clob_ref_map(|_c| ())?.unwrap();
        }
        Null => {}
      }
    } else if cursor.depth() > 0 { // it was `None`
      cursor.step_out()?;
    } else {
      break;
    }
  }
  println!("Skimmed {} values", count);
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match skim_file(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}