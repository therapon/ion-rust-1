use types::*;

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
#[derive(Debug)]
pub struct SymbolTable {
  // Strings in the table are nullable
  symbols: Vec<Option<String>>,
}

impl SymbolTable {
  pub fn new() -> SymbolTable {
    let mut symbols = Vec::with_capacity(SYSTEM_SYMBOLS_1_0.len() + 1);
    symbols.push(None); // $0
    symbols.extend(SYSTEM_SYMBOLS_1_0.iter().map(|s| Some(s.to_string())));
    SymbolTable {
      symbols
    }
  }

  pub fn intern(&mut self, nullable_text: Option<String>) -> usize {
    self.symbols.push(nullable_text);
    self.symbols.len()
  }

  pub fn resolve<I>(&self, index: I) -> Option<&str> where I: Into<IonSymbolId> {
    let index: usize = index.into().into();
    if index >= self.symbols.len() {
      return None;
    }

    match &self.symbols[index] {
      Some(text) => Some(text.as_ref()),
      None => None
    }
  }
}