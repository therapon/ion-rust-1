use std::convert::From;
use std::io;

#[derive(Debug, Fail)]
pub enum IonError {
  #[fail(display = "An IO error occurred: {}", description)]
  IoError {
    description: String,
  },

  #[fail(display = "A decoding error occurred: {}", description)]
  DecodingError {
    description: String
  }
}

pub fn decoding_error<T>(description: &str) -> Result<T, IonError> {
  Err(IonError::DecodingError {
    description: description.to_string()
  })
}

pub fn io_error<T>(description: &str) -> Result<T, IonError> {
  Err(IonError::IoError {
    description: description.to_string()
  })
}

impl From<io::Error> for IonError {
  fn from(error: io::Error) -> Self {
    use std::error::Error;
    IonError::IoError {
      description: format!("Encountered an IO error: {:?}", error.description())
    }
  }
}