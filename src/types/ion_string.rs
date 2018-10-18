use std::convert::From;
use std::convert::AsRef;
use std::borrow::Cow;
use std::ops::Deref;

// &str-based value, requires no copying

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonStringRef<'a> {
  text: &'a str
}

impl <'a> Deref for IonStringRef<'a> {
  type Target = str;

  fn deref(&self) -> &str {
    self.text
  }
}

impl <'a> AsRef<str> for IonStringRef<'a> {
  fn as_ref(&self) -> &str {
    self.text
  }
}

impl <'a, 'b: 'a> From<&'b str> for IonStringRef<'a> {
  fn from(text: &'b str) -> IonStringRef<'a> {
    IonStringRef {
      text
    }
  }
}

impl <'a> From<IonStringRef<'a>> for String {
  fn from(ion_string: IonStringRef) -> String {
    ion_string.text.to_string()
  }
}

// Owned String, requires copying from the source buffer

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonString {
  text: String
}

impl Deref for IonString {
  type Target = str;

  fn deref(&self) -> &str {
    &self.text
  }
}

impl AsRef<str> for IonString {
  fn as_ref(&self) -> &str {
    &self.text
  }
}

impl From<IonString> for String {
  fn from(ion_string: IonString) -> String {
    ion_string.text
  }
}