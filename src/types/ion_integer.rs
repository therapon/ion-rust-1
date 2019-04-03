use std::convert::From;

//TODO: This value's internal representation should be flexible enough to support BigInteger-style
// values. For now, we're just starting with i64.

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct IonInteger {
    value: i64,
}

impl From<u64> for IonInteger {
    fn from(value: u64) -> Self {
        IonInteger {
            value: value as i64,
        }
    }
}

impl From<i64> for IonInteger {
    fn from(value: i64) -> Self {
        IonInteger { value }
    }
}

impl From<usize> for IonInteger {
    fn from(value: usize) -> Self {
        IonInteger {
            value: value as i64,
        }
    }
}

impl From<isize> for IonInteger {
    fn from(value: isize) -> Self {
        IonInteger {
            value: value as i64,
        }
    }
}

impl From<IonInteger> for u64 {
    fn from(ion_integer: IonInteger) -> Self {
        ion_integer.value as u64
    }
}

impl From<IonInteger> for i64 {
    fn from(ion_integer: IonInteger) -> Self {
        ion_integer.value
    }
}

impl From<IonInteger> for isize {
    fn from(ion_integer: IonInteger) -> Self {
        ion_integer.value as isize
    }
}

impl From<IonInteger> for usize {
    fn from(ion_integer: IonInteger) -> Self {
        ion_integer.value as usize
    }
}
