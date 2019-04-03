extern crate ion_rust;

use ion_rust::binary::ion_cursor::IonDataSource;
use ion_rust::binary::BinaryIonReader;
use ion_rust::result::IonResult;
use ion_rust::types::IonType;

use num_traits::cast::ToPrimitive;
use std::fs::File;

fn skim_file(path: &str) -> IonResult<()> {
    let file = File::open(path).expect("Unable to open file");
    //  let buf_reader = std::io::BufReader::new(file);
    let buf_reader = std::io::BufReader::with_capacity(1024 * 32, file);
    let mut cursor = BinaryIonReader::new(buf_reader)?;
    skim_values(&mut cursor)
}

fn skim_values<R: IonDataSource>(reader: &mut BinaryIonReader<R>) -> IonResult<()> {
    use crate::IonType::*;
    let mut count = 0;
    let mut accumulator: i64 = 20190403; // Using signed integer for easy comparison with Java
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
                }
                String => {
                    //let _text = reader.string_ref_map(|_s| ())?.unwrap();
                    accumulator ^= reader
                        .string_ref_map(|s| s.chars().next().unwrap_or(0 as char) as i64)?
                        .unwrap();
                    //          let _text = reader.read_string()?.unwrap();
                }
                Symbol => {
                    accumulator ^= reader.read_symbol_id()?.unwrap() as i64;
                }
                Integer => {
                    accumulator ^= reader.read_i64()?.unwrap();
                }
                Float => {
                    accumulator ^= reader.read_f64()?.unwrap() as i64;
                }
                Decimal => {
                    let bd = reader.read_decimal()?.unwrap();
                    accumulator ^= bd.to_i64().unwrap();
                }
                Timestamp => {
                  let _timestamp = reader.read_timestamp()?.unwrap();
                }
                Boolean => {
                    if reader.read_bool()?.unwrap() {
                        accumulator = accumulator ^ 8675309;
                    } else {
                        accumulator = accumulator ^ 24601;
                    }
                }
                Blob => {
                    let _blob = reader.blob_ref_map(|_b| ())?.unwrap();
                }
                Clob => {
                    let _clob = reader.clob_ref_map(|_c| ())?.unwrap();
                }
                Null => {}
            }
        } else if reader.depth() > 0 {
            // it was `None`
            reader.step_out()?;
        } else {
            break;
        }
    }
    println!("Accumulator: {}", accumulator);
    println!("Skimmed {} values", count);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let default_file =
        "/Users/zslayton/local_ion_data/ion_data2/item_change_listener.shorthand.log.2018-07-27-17";
    let path = args.get(1).map(|s| s.as_ref()).unwrap_or(default_file);

    //  let path = "EC2_EFFICIENCY.10n";
    match skim_file(path) {
        Ok(_) => {}
        Err(error) => panic!("Failed to read the file: {:?}", error),
    }
}