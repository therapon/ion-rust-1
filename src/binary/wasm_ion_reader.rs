use binary::ion_reader::BinaryIonReader;
use wasm_bindgen::prelude::*;
use std::io;
use result::IonResult;

use cfg_if::cfg_if;
use wasm_bindgen::prelude::*;

// Sadly, we cannot borrow a slice of bytes from across the JS/Rust language boundary
// because wasm_bindgen cannot currently wrap structs that have a lifetime.
#[wasm_bindgen]
pub struct WasmBinaryIonReader {
  reader: BinaryIonReader<io::Cursor<Vec<u8>>>
}

#[wasm_bindgen]
impl WasmBinaryIonReader {
  pub fn new(data: &[u8]) -> WasmBinaryIonReader {
    let mut buffer: Vec<u8> = Vec::with_capacity(data.len());
    buffer.extend_from_slice(data);
    let io_cursor = io::Cursor::new(buffer);
    let reader: BinaryIonReader<io::Cursor<Vec<u8>>> = BinaryIonReader::new(io_cursor)
      .expect("Initialization failed. Input does not appear to be valid binary Ion.");
    WasmBinaryIonReader {
      reader
    }
  }
}