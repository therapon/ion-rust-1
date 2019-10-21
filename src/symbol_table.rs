use types::*;
use std::rc::Rc;

const SYSTEM_SYMBOLS_1_0: &[&str] = &[
  // $0,
  "$ion",
  "$ion_1_0",
  "$ion_symbol_table",
  "name",
  "version",
  "imports",
  "symbols",
  "max_id",
  "$ion_shared_symbol_table"
];

//TODO: IonString instead of String
//TODO: Optimize by storing Vec<Rc<String>> so other dom values can point to the same String?
//TODO: Optimize for writing as well by adding a HashMap
#[derive(Debug, Default)]
pub struct SymbolTable {
  // Strings in the table are nullable
  symbols: Vec<Option<Rc<String>>>,
}

impl SymbolTable {
  pub fn new() -> SymbolTable {
    let mut symbols = Vec::with_capacity(SYSTEM_SYMBOLS_1_0.len() + 1);
    symbols.push(None); // $0
    symbols.extend(SYSTEM_SYMBOLS_1_0.iter().map(|s| Some(Rc::new(s.to_string()))));
    SymbolTable {
      symbols
    }
  }

  pub fn intern(&mut self, nullable_text: Option<String>) -> usize {
    self.symbols.push(nullable_text.map(Rc::new));
    self.symbols.len()
  }

  //TODO: Return an IonSymbol instead of an Option<RcString>
  //TODO: Or: return an Option<&Rc<String>> so the recipient can decide whether to clone it
  pub fn resolve<I>(&self, index: I) -> Option<Rc<String>> where I: Into<IonSymbolId> {
    let index: usize = index.into().into();
    if index >= self.symbols.len() {
      return None;
    }

    match &self.symbols[index] {
      Some(rc_text) => Some(Rc::clone(rc_text)),
      None => None
    }
  }

  pub fn len(&self) -> usize {
    self.symbols.len()
  }

  pub fn iter(&self) -> impl Iterator<Item=(usize, Option<&str>)> {
    self.symbols
        .iter()
        .enumerate()
        .map(|(index, text_opt)| (index+1, text_opt.as_ref().map(|text| text.as_str())) )
  }
}