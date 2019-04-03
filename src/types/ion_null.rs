use crate::types::ion_type::IonType;
use std::convert::From;

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone)]
pub struct IonNull {
    ion_type: IonType,
}

impl IonNull {
    fn ion_type(&self) -> IonType {
        self.ion_type
    }
}

impl From<IonType> for IonNull {
    fn from(ion_type: IonType) -> Self {
        IonNull { ion_type }
    }
}
