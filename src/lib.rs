#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

extern crate bytes;
extern crate failure;
extern crate env_logger;

#[macro_use] extern crate failure_derive;
#[macro_use] extern crate log;

mod header_byte;
mod errors;
mod ion_cursor;
mod ion_type;
mod ion_type_code;
mod uint;
mod var_uint;
