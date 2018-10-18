use std::convert::From;
use types::ion_integer::IonInteger;
use types::ion_string::IonString;

pub type IonSymbolId = IonInteger;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IonSymbol {
  id: IonSymbolId,
  text: Option<IonString> // May or may not have been resolved successfully
}

impl IonSymbol {
  pub fn id(&self) -> IonSymbolId {
    self.id
  }

  pub fn text(&self) -> &Option<IonString> {
    &self.text
  }
}

impl IonSymbol {
  fn new<I, S>(id: I, text: Option<S>) -> Self where I: Into<IonInteger>,
                                                     S: Into<IonString> {
    IonSymbol {
      id: Into::into(id),
      text: text.map(Into::into)
    }
  }
}