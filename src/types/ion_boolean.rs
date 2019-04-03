use std::convert::From;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct IonBoolean {
    value: bool,
}

impl IonBoolean {
    pub fn is_true(&self) -> bool {
        self.value
    }

    pub fn is_false(&self) -> bool {
        !self.value
    }

    pub fn boolean_value(&self) -> bool {
        self.value
    }
}

impl From<bool> for IonBoolean {
    fn from(value: bool) -> Self {
        IonBoolean { value }
    }
}
