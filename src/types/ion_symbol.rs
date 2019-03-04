use std::convert::From;
use types::IonInteger;
use std::rc::Rc;

pub type IonSymbolId = IonInteger;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Hash)]
pub struct IonSymbol {
  id: IonSymbolId,
  text: Option<Rc<String>> // May or may not have been resolved successfully
}

// TODO: Should IonSymbol be a trait to allow for Rc, Arc, versions if needed?
impl IonSymbol {
  pub fn id(&self) -> IonSymbolId {
    self.id
  }

  pub fn text(&self) -> Option<&str> {
    self.text.as_ref().map(|t| t.as_ref().as_ref()) // RC -> &String -> &str
  }
}

impl IonSymbol {
  //TODO: Remove 'Rc' from this signature
  pub fn new<I>(id: I, text: Option<Rc<String>>) -> Self where I: Into<IonInteger> {
    IonSymbol {
      id: Into::into(id),
      text
    }
  }
}

impl From<IonSymbol> for (IonSymbolId, Option<Rc<String>>) {
  fn from(ion_symbol: IonSymbol) -> Self {
    (ion_symbol.id, ion_symbol.text)
  }
}