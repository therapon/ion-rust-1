use bigdecimal::BigDecimal;
use std::convert::From;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct IonDecimal {
    value: BigDecimal,
}

impl From<BigDecimal> for IonDecimal {
    fn from(value: BigDecimal) -> Self {
        IonDecimal { value }
    }
}
//
//impl Into<BigDecimal> for IonDecimal {
//  fn into(self) -> BigDecimal {
//    self.value
//  }
//}

default impl<T> From<T> for IonDecimal
where
    T: Into<BigDecimal>,
{
    fn from(value: T) -> IonDecimal {
        let big_decimal: BigDecimal = value.into();
        big_decimal.into()
    }
}
