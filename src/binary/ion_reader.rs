use binary::ion_cursor::BinaryIonCursor;
use binary::ion_cursor::IonDataSource;
use result::*;
use types::*;
use bigdecimal::BigDecimal;
use chrono::DateTime;
use chrono::FixedOffset;
use std::rc::Rc;
use ::ion_system::IonSystem;
use ::symbol_table::SymbolTable;

pub struct BinaryIonReader<D: IonDataSource> {
  pub symbols: SymbolTable,
  pub cursor: BinaryIonCursor<D>,
  pub ion_system: IonSystem
}

impl <D: IonDataSource> BinaryIonReader<D> {
  pub fn new(data_source: D) -> IonResult<Self> {
    Ok(BinaryIonReader {
      symbols: SymbolTable::new(),
      cursor: BinaryIonCursor::new(data_source)?,
      ion_system: IonSystem::new()
    })
  }

  pub fn symbol_table(&self) -> &SymbolTable {
    &self.symbols
  }

  #[inline]
  pub fn next(&mut self) -> IonResult<Option<IonType>> {
    let ion_type_option = self.cursor.next()?;
    if self.cursor.value_is_symbol_table() {
      self.read_local_symbol_table()?;
      return self.next();
    }
    Ok(ion_type_option)
  }

  fn read_local_symbol_table(&mut self) -> IonResult<()> {
//    println!("Reading LST!");
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
//    println!("Done reading.");
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
      match self.read_string()? {
        Some(symbol_text) => {
          //println!("Interning {:?}", &symbol_text);
          self.symbols.intern(Some(symbol_text.into()));
        }
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

  //TODO:
  // pub fn read_ion_value(&mut self) -> IonResult<IonValue>

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

    // Because we've tested the ion type and checked for null, we can unwrap all of the Option<_> values
    let ion_value = match ion_type {
      Null => unreachable!("Cannot have a null-typed value for which is_null() returns false."),
      Boolean => IonValue::Boolean(self.read_bool()?.unwrap().into()),
      Integer => IonValue::Integer(self.unchecked_read_i64()?.unwrap().into()),
//      Integer => IonValue::Integer(self.read_i64()?.unwrap().into()),
      Float => IonValue::Float(self.read_f64()?.unwrap().into()),
      Decimal => IonValue::Decimal(self.read_decimal()?.unwrap().into()),
//      Timestamp => IonValue::Timestamp(self.read_timestamp()?.unwrap().into()),
      //TODO: Restore this after fixing Timestamp
      Timestamp => IonValue::Null(IonNull::from(ion_type)),
      Symbol => IonValue::Symbol(self.read_symbol()?.unwrap().into()),
      String => {
        let mut text = self.ion_system.string_from_pool();

        let text = self
          .string_ref_map(move |s| {
            text.push_str(s);
            text
          })?
          .unwrap();

        IonValue::String(IonString::new(text))
      },
      Clob => IonValue::Clob(self.read_clob_bytes()?.unwrap().into()),
      Blob => IonValue::Blob(self.read_blob_bytes()?.unwrap().into()),
      Struct => IonValue::Struct(self.unchecked_struct_value()?.into()),
      List => IonValue::List(self.unchecked_list_value()?.into()),
      SExpression => IonValue::SExpression(self.s_expression_value()?.unwrap().into()),
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
  pub fn struct_value(&mut self) -> IonResult<Option<IonStruct>> {
    if self.ion_type() != IonType::Struct ||  self.is_null() {
      return Ok(None);
    }

    self.unchecked_struct_value().map(|v| Some(v))
  }

  fn unchecked_struct_value(&mut self) -> IonResult<IonStruct> {
    let mut struct_builder = self.ion_system.ion_struct_builder();
    self.step_in()?;
    while let Some(_ion_type) = self.next()? {
      let ion_dom_value = self.ion_dom_value()?;
      struct_builder.add_child(ion_dom_value);
    }
    self.step_out()?;

    let ion_struct = struct_builder.build();
    Ok(ion_struct)
  }

  pub fn list_value(&mut self) -> IonResult<Option<IonList>> {
    if self.ion_type() != IonType::List || self.is_null() {
      return Ok(None);
    }

    self.unchecked_list_value().map(|v| Some(v))
  }

  pub fn unchecked_list_value(&mut self) -> IonResult<IonList> {
    let mut list_builder = self.ion_system.ion_list_builder();
    self.step_in()?;
    while let Some(_ion_type) = self.next()? {
      let ion_dom_value = self.ion_dom_value()?;
      list_builder.add_child(ion_dom_value);
    }
    self.step_out()?;

    Ok(list_builder.build())
  }

  pub fn s_expression_value(&mut self) -> IonResult<Option<IonSExpression>> {
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
  pub fn read_bool(&mut self) -> IonResult<Option<bool>> {
    self.cursor.read_bool()
  }

  pub fn read_i64(&mut self) -> IonResult<Option<i64>> {
    self.cursor.read_i64()
  }

  pub fn unchecked_read_i64(&mut self) -> IonResult<Option<i64>> {
    self.cursor.unchecked_read_i64()
  }

  pub fn read_f32(&mut self) -> IonResult<Option<f32>> {
    self.cursor.read_f32()
  }

  pub fn read_f64(&mut self) -> IonResult<Option<f64>> {
    self.cursor.read_f64()
  }

  pub fn read_decimal(&mut self) -> IonResult<Option<BigDecimal>> {
    self.cursor.read_decimal()
  }

  pub fn read_timestamp(&mut self) -> IonResult<Option<DateTime<FixedOffset>>> {
    self.cursor.read_timestamp()
  }

  pub fn read_string(&mut self) -> IonResult<Option<String>> {
    self.cursor.read_string()
  }

  pub fn string_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&str) -> T {
    self.cursor.string_ref_map(f)
  }

  pub fn read_symbol_id(&mut self) -> IonResult<Option<IonSymbolId>> {
    self.cursor.read_symbol_id()
  }

  pub fn read_symbol(&mut self) -> IonResult<Option<IonSymbol>> {
    let symbol_id = self.read_symbol_id()?.unwrap();
    let resolved_text = self.symbols.resolve(symbol_id);
    let symbol = IonSymbol::new(symbol_id, resolved_text);
    Ok(Some(symbol))
  }

  pub fn read_blob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
    self.cursor.read_blob_bytes()
  }

  pub fn blob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&[u8]) -> T {
    self.cursor.blob_ref_map(f)
  }

  pub fn read_clob_bytes(&mut self) -> IonResult<Option<Vec<u8>>> {
    self.cursor.read_clob_bytes()
  }

  pub fn clob_ref_map<F, T>(&mut self, f: F) -> IonResult<Option<T>> where F: FnOnce(&[u8]) -> T {
    self.cursor.clob_ref_map(f)
  }

  pub fn annotation_ids<'a>(&'a self) -> impl Iterator<Item=IonSymbolId> + 'a {
    self.cursor.annotation_ids()
  }

  pub fn annotations(& self) -> IonResult<Vec<IonSymbol>> {
    let annotation_ids = self.annotation_ids();
    let mut resolved_symbols = Vec::new();
    for annotation_id in annotation_ids {
      let text: Rc<String> = match self.symbols.resolve(annotation_id) {
        Some(text) => text,
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
      None => decoding_error(
        format!("Could not resolve field symbol ID {:?}", field_id)
      )
    }
  }

  //TODO: This should return an Option like everything else
  pub fn read_text(&mut self) -> IonResult<String> {
    let ion_type = self.cursor.ion_type();
    match ion_type {
      IonType::String => Ok(self.cursor.read_string()?.unwrap()),
      IonType::Symbol => {
        let symbol_id = self.cursor.read_symbol_id()?.unwrap();
        //println!("Resolving: {:?} in {:?}, {} symbols", symbol_id, self.symbols, self.symbols.symbols.len());
        match self.symbols.resolve(symbol_id) {
          Some(text) => Ok(text.as_ref().to_string()), // This clones the SymbolTable entry
          None => decoding_error(
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