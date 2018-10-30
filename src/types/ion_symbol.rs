use std::convert::From;
use types::IonInteger;
use types::IonString;

pub type IonSymbolId = IonInteger;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
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
  pub fn new<I, S>(id: I, text: Option<S>) -> Self where I: Into<IonInteger>,
                                                     S: Into<IonString> {
    IonSymbol {
      id: Into::into(id),
      text: text.map(Into::into)
    }
  }
}

impl From<IonSymbol> for (IonSymbolId, Option<IonString>) {
  fn from(ion_symbol: IonSymbol) -> Self {
    (ion_symbol.id, ion_symbol.text)
  }
}