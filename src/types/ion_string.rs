use lifeguard::RcRecycled;
use std::convert::AsRef;
use std::convert::From;
use std::fmt::Debug;
use std::fmt::Error;
use std::fmt::Formatter;
use std::ops::Deref;

//// &str-based value, requires no copying
//
//#[derive(PartialEq, Eq, PartialOrd, Clone)]
//pub struct IonStringRef<'a> {
//  text: &'a str
//}
//
//impl <'a> Debug for IonStringRef<'a> {
//  fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
//    write!(f, "\"{}\"", self.text)?;
//    Ok(())
//  }
//}
//
//impl <'a> Deref for IonStringRef<'a> {
//  type Target = str;
//
//  fn deref(&self) -> &str {
//    self.text
//  }
//}
//
//impl <'a> AsRef<str> for IonStringRef<'a> {
//  fn as_ref(&self) -> &str {
//    self.text
//  }
//}
//
//impl <'a, 'b: 'a> From<&'b str> for IonStringRef<'a> {
//  fn from(text: &'b str) -> IonStringRef<'a> {
//    IonStringRef {
//      text
//    }
//  }
//}
//
//impl <'a> From<IonStringRef<'a>> for String {
//  fn from(ion_string: IonStringRef) -> String {
//    ion_string.text.to_string()
//  }
//}

// Owned String, requires copying from the source buffer

#[derive(PartialEq, Eq, PartialOrd, Clone)]
pub struct IonString {
    text: RcRecycled<String>,
}

impl Debug for IonString {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "\"{}\"", self.text)?;
        Ok(())
    }
}

impl IonString {
    pub fn new(text: RcRecycled<String>) -> IonString {
        IonString { text }
    }
}

impl Deref for IonString {
    type Target = str;

    fn deref(&self) -> &str {
        self.text.as_ref()
    }
}

impl AsRef<str> for IonString {
    fn as_ref(&self) -> &str {
        self.text.as_ref()
    }
}

//impl From<&str> for IonString {
//  fn from(text: &str) -> IonString {
//    text.to_string().into()
//  }
//}

impl From<IonString> for String {
    fn from(ion_string: IonString) -> String {
        ion_string.text.detach()
    }
}
//
//impl From<String> for IonString {
//  fn from(text: String) -> IonString {
//    IonString {
//      text
//    }
//  }
//}
