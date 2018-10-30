use std::io::Read;
use binary::ion_cursor::BinaryIonCursor;
use binary::ion_cursor::IonDataSource;
use binary::symbol_table::SymbolTable;
use result::*;
use types::*;
use std::collections::BTreeMap;

pub struct BinaryIonReader<D: IonDataSource> {
  pub symbols: SymbolTable,
  pub cursor: BinaryIonCursor<D>
}

impl <D: IonDataSource> BinaryIonReader<D> {
  pub fn new(data_source: D) -> IonResult<BinaryIonReader<D>> {
    Ok(BinaryIonReader {
      symbols: SymbolTable::new(),
      cursor: BinaryIonCursor::new(data_source)?
    })
  }

  pub fn next(&mut self) -> IonResult<Option<IonType>> {
    let ion_type_option = self.cursor.next()?;
    if self.cursor.value_is_symbol_table() {
      self.read_local_symbol_table()?;
      return self.cursor.next();
    }
    Ok(ion_type_option)
  }

  fn read_local_symbol_table(&mut self) -> IonResult<()> {
    self.step_in()?;
    loop {
      let ion_type = self.next()?;
      if ion_type.is_none() {
        break;
      }
      match self.field_id().map(Into::into) {
        Some(6u64) /*imports*/ => self.read_symbol_table_imports()?,
        Some(7u64) /*symbols*/ => self.read_symbol_table_symbols()?,
        _ => continue
      }
    }
    self.step_out()?;
    Ok(())
  }

  fn read_symbol_table_imports(&mut self) -> IonResult<()> {Ok(())}

  fn read_symbol_table_symbols(&mut self) -> IonResult<()> {
    self.step_in()?;
    while let Some(ion_type) = self.next()? {
      if ion_type != IonType::String {
        return decoding_error(
          format!("Found a {:?} value in the $ion_symbol_table symbols list", ion_type)
        );
      }
      match self.string_value()? {
        Some(symbol_text) => self.symbols.intern(Some(symbol_text.into())),
        None => continue
      };
    }
    self.step_out()?;
    Ok(())
  }

  pub fn step_in(&mut self) -> IonResult<()> {
    self.cursor.step_in()
  }

  pub fn step_out(&mut self) -> IonResult<()> {
    self.cursor.step_out()
  }

  pub fn ion_dom_value(&mut self) -> IonResult<IonDomValue> {
    use self::IonType::*;
    use self::IonValue;
    let ion_type = self.ion_type();
    let field = self.field()?;
    let annotations = self.annotations()?;

    if self.is_null() {
      let dom_value = IonDomValue::new(
        field,
        annotations,
        IonValue::Null(IonNull::from(ion_type))
      );
      return Ok(dom_value);
    }

    let ion_value = match ion_type {
      Null => unreachable!("Cannot have a null-typed value for which is_null() returns false."),
      Boolean => IonValue::Boolean(self.boolean_value()?.unwrap()),
      Integer => IonValue::Integer(self.integer_value()?.unwrap()),
      Float => IonValue::Float(self.float_value()?.unwrap()),
      Decimal => IonValue::Decimal(self.decimal_value()?.unwrap()),
      Timestamp => IonValue::Timestamp(self.timestamp_value()?.unwrap()),
      Symbol => IonValue::Symbol(IonSymbol::new::<IonSymbolId, IonString>(self.symbol_id_value()?.unwrap(), None)),
      String => IonValue::String(self.string_value()?.unwrap()),
      Clob => IonValue::Clob(self.clob_value()?.unwrap()),
      Blob => IonValue::Blob(self.blob_value()?.unwrap()),
      Struct => IonValue::Struct(self.struct_value()?.unwrap()),
      List => IonValue::List(self.list_value()?.unwrap()),
      SExpression => IonValue::SExpression(self.s_expression_value()?.unwrap()),
//      _ => unimplemented!("Can't turn a {:?} into an IonValue yet. :(", ion_type)
    };
    Ok(IonDomValue::new(field, annotations, ion_value))
  }

  pub fn is_null(&self) -> bool {
    self.cursor.is_null()
  }

  pub fn ion_type(&self) -> IonType {
    //TODO: If next() returns None, this doesn't get updated.
    self.cursor.ion_type()
  }

  pub fn depth(&self) -> usize {
    self.cursor.depth()
  }

  // ---- Composite Types --------------------------
  fn struct_value(&mut self) -> IonResult<Option<IonStruct>> {
    if self.ion_type() != IonType::Struct ||  self.is_null() {
      return Ok(None);
    }
    let mut map = BTreeMap::new(); // No allocations initially
    self.step_in()?;
    while let Some(_ion_type) = self.next()? {
      let field_symbol = self.field()?.expect("Missing field ID inside struct.");
      let (_field_id, field_name): (IonSymbolId, Option<IonString>) = field_symbol.into();
      let ion_dom_value = self.ion_dom_value()?;
      map.insert(field_name.unwrap().into(), ion_dom_value);
    }
    self.step_out()?;

    let ion_struct = IonStruct::from(map);
    Ok(Some(ion_struct))
  }

  fn list_value(&mut self) -> IonResult<Option<IonList>> {
    if self.ion_type() != IonType::List || self.is_null() {
      return Ok(None);
    }

    let mut list = Vec::new();
    self.step_in()?;
    while let Some(_ion_type) = self.next()? {
      let ion_dom_value = self.ion_dom_value()?;
      list.push(ion_dom_value);
    }
    self.step_out()?;

    let ion_list = IonList::from(list);
    Ok(Some(ion_list))
  }

  fn s_expression_value(&mut self) -> IonResult<Option<IonSExpression>> {
    if self.ion_type() != IonType::SExpression || self.is_null() {
      return Ok(None);
    }

    let mut sexp = Vec::new();
    self.step_in()?;
    while let Some(_ion_type) = self.next()? {
      let ion_dom_value = self.ion_dom_value()?;
      sexp.push(ion_dom_value);
    }
    self.step_out()?;

    let ion_sexpression = IonSExpression::from(sexp);
    Ok(Some(ion_sexpression))
  }

  // ---- Scalar Types --------------------------
  pub fn boolean_value(&mut self) -> IonResult<Option<IonBoolean>> {
    self.cursor.boolean_value()
  }

  pub fn integer_value(&mut self) -> IonResult<Option<IonInteger>> {
    self.cursor.integer_value()
  }

  pub fn float_value(&mut self) -> IonResult<Option<IonFloat>> {
    self.cursor.float_value()
  }

  pub fn decimal_value(&mut self) -> IonResult<Option<IonDecimal>> {
    self.cursor.decimal_value()
  }

  pub fn timestamp_value(&mut self) -> IonResult<Option<IonTimestamp>> {
    self.cursor.timestamp_value()
  }

  pub fn string_value(&mut self) -> IonResult<Option<IonString>> {
    self.cursor.string_value()
  }

  pub fn string_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonStringRef) -> T {
    self.cursor.string_ref_map(f)
  }

  pub fn symbol_id_value(&mut self) -> IonResult<Option<IonSymbolId>> {
    self.cursor.symbol_id_value()
  }

  pub fn blob_value(&mut self) -> IonResult<Option<IonBlob>> {
    self.cursor.blob_value()
  }

  pub fn blob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonBlobRef) -> T {
    self.cursor.blob_ref_map(f)
  }

  pub fn clob_value(&mut self) -> IonResult<Option<IonClob>> {
    self.cursor.clob_value()
  }

  pub fn clob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: Fn(IonClobRef) -> T {
    self.cursor.clob_ref_map(f)
  }

  pub fn annotation_ids<'a>(&'a self) -> impl Iterator<Item=IonSymbolId> + 'a {
    self.cursor.annotation_ids()
  }

  //TODO: This allocates a Vec each time. Optimize?
  pub fn annotations<'a>(&'a self) -> IonResult<Vec<IonSymbol>> {
    let annotation_ids = self.annotation_ids();
    let mut resolved_symbols: Vec<IonSymbol> = Vec::with_capacity(annotation_ids.size_hint().0);
    for annotation_id in annotation_ids {
      let text: IonString = match self.symbols.resolve(annotation_id) {
        Some(text) => text.into(),
        None => return decoding_error(
          format!("Could not resolve annotation symbol ID {:?}", annotation_id)
        )
      };
      resolved_symbols.push(IonSymbol::new(annotation_id, Some(text)));
    }
    Ok(resolved_symbols)
  }

  pub fn field_id(&self) -> Option<IonSymbolId> {
    self.cursor.field_id()
  }

  pub fn field(&self) -> IonResult<Option<IonSymbol>> {
    if self.field_id().is_none() {
      return Ok(None);
    }
    let field_id = self.field_id().unwrap();
    match self.symbols.resolve(field_id) {
      Some(text) => Ok(Some(IonSymbol::new(field_id, Some(text)))),
      None => return decoding_error(
        format!("Could not resolve field symbol ID {:?}", field_id)
      )
    }
  }

  fn read_text(&mut self) -> IonResult<IonString> {
    let ion_type = self.cursor.ion_type();
    match ion_type {
      IonType::String => Ok(self.cursor.string_value()?.unwrap()),
      IonType::Symbol => {
        let symbol_id = self.cursor.symbol_id_value()?.unwrap();
        //println!("Resolving: {:?} in {:?}, {} symbols", symbol_id, self.symbols, self.symbols.symbols.len());
        match self.symbols.resolve(symbol_id) {
          Some(text) => Ok(text.into()),
          None => return decoding_error(
            format!("Could not resolve symbol ID {:?}", symbol_id)
          )
        }
      },
      _ => panic!("Tried to get text from a {:?}", ion_type)
    }
  }

  pub fn ion_dom_values<'a>(&'a mut self) -> impl Iterator<Item=IonResult<IonDomValue>> + 'a {
    BinaryIonCursorDomValues {
      reader: self
    }
  }
}

struct BinaryIonCursorDomValues<'a, R: IonDataSource> {
  reader: &'a mut BinaryIonReader<R>
}

impl <'a, R> Iterator for BinaryIonCursorDomValues<'a, R> where R: IonDataSource {
  type Item = IonResult<IonDomValue>;
  fn next(&mut self) -> Option<<Self as Iterator>::Item> {
    let _ion_type = match self.reader.next() {
      Ok(Some(ion_type)) => ion_type,
      Ok(None) => return None,
      Err(error) => return Some(Err(error))
    };
    Some(self.reader.ion_dom_value())
  }
}