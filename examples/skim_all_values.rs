extern crate amzn_ion;

use amzn_ion::binary::ion_cursor::{IonDataSource, BinaryIonCursor};
use amzn_ion::result::IonResult;
use amzn_ion::types::ion_type::IonType;

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io;

fn skim_file(path: &str) -> IonResult<()> {
  let file = File::open(path).expect("Unable to open file");
  let mut reader = BufReader::with_capacity(32 * 1_024, file);
  let mut cursor = BinaryIonCursor::new(&mut reader)?;

  skim_values(&mut cursor, 0)
}

fn skim_values<R: IonDataSource>(cursor: &mut BinaryIonCursor<R>, depth: usize) -> IonResult<()> {
  let mut current_ion_type: Option<IonType> = cursor.next()?;
  while let Some(ion_type) = current_ion_type {
    { // Annotations scope
      let annotations = cursor.annotations();
      let num_annotations = annotations.count();
      let field_id = cursor.field_id();
      match (num_annotations, field_id) {
        (0, Some(ref field_id)) => {
          //debug!("{} Found a(n) {:?} with field_id: {:?}", marker, ion_type, field_id);
        },
        (0, None) => {
          //debug!("{} Found a(n) {:?}", marker, ion_type);
        },
        (_, Some(ref field_id)) => {
          //debug!("{} Found a(n) {:?} with field_id: {:?} and annotations: {:?}",
//                     marker,
//                     ion_type,
//                     field_id,
//                     annotations);
        },
        (_, None) => {
          ////debug!("{} Found a(n) {:?} with annotations: {:?}",
//                     marker,
//                     ion_type,
//                     annotations);
        },
      }
    }

    if cursor.is_null() {
      ////debug!("  VALUE: null.{:?}", ion_type);
    } else {
      use self::IonType::*;
      match ion_type {
        String => {
          let text = cursor.string_ref_value()?.unwrap();
          ////debug!("  VALUE: {}", text);
        },
        Symbol => {
          let symbol_id = cursor.symbol_id_value()?.unwrap();
          ////debug!("  VALUE: {}", symbol_id);
        },
        Integer => {
          let int = cursor.integer_value()?.unwrap();
          ////debug!("  VALUE: {}", int)
        },
        Boolean => {
          let boolean = cursor.boolean_value()?.unwrap();
          ////debug!("  VALUE: {}", boolean)
        },
        Float => {
          let float = cursor.float_value()?.unwrap();
          ////debug!("  VALUE: {}", float)
        },
        Blob => {
          let blob = cursor.blob_ref_value()?.unwrap();
        },
        Clob => {
          let clob = cursor.clob_ref_value()?.unwrap();
        }
        _ => {
          ////debug!("  VALUE: <{:?} not yet supported>", ion_type);
        }
      }
    }

    if ion_type == IonType::List || ion_type == IonType::Struct {
      cursor.step_in()?;
      {
        let _ = skim_values(cursor, depth + 1)?;
      }
      cursor.step_out()?;
    }
    current_ion_type = cursor.next()?;
  }
  Ok(())
}

fn main() {
  let path = "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
  match skim_file(path) {
    Ok(_) => {},
    Err(error) => panic!("Failed to read the file: {:?}", error)
  }
}