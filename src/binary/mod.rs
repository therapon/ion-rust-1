mod header_byte;
mod ion_reader;
mod ion_writer;
pub mod ion_cursor;
mod ion_type_code;
mod uint;
mod int;
mod var_uint;
mod var_int;
//mod wasm_ion_reader;

pub use self::ion_reader::BinaryIonReader;
//pub use self::wasm_ion_reader::WasmBinaryIonReader;