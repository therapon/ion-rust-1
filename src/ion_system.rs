use crate::types::*;
use lifeguard::*;

pub struct IonSystem {
    pub dom_vec_pool: Pool<Vec<IonDomValue>>,
    pub string_pool: Pool<String>,
}

impl IonSystem {
    pub fn new() -> IonSystem {
        IonSystem {
            dom_vec_pool: pool()
                .with(StartingSize(16))
                .with(MaxSize(32))
                .with(Supplier(|| Vec::with_capacity(32)))
                .build(),
            string_pool: pool()
                .with(StartingSize(16))
                .with(MaxSize(32))
                .with(Supplier(|| String::with_capacity(256)))
                .build(),
        }
    }

    pub fn string_from_pool(&self) -> RcRecycled<String> {
        self.string_pool.new_rc()
    }

    pub fn new_ion_string<S: AsRef<str>>(&self, text: S) -> IonString {
        IonString::new(self.string_pool.new_rc_from(text))
    }

    //TODO: module nested inside of IonStruct type module?
    pub fn ion_struct_builder(&mut self) -> IonStructBuilder {
        let children = self.dom_vec_pool.new_rc();
        IonStructBuilder { children }
    }

    pub fn ion_list_builder(&mut self) -> IonListBuilder {
        let children = self.dom_vec_pool.new_rc();
        IonListBuilder { children }
    }
}

pub struct IonStructBuilder {
    children: RcRecycled<Vec<IonDomValue>>,
}

impl IonStructBuilder {
    #[inline]
    pub fn add_child(&mut self, child: IonDomValue) {
        self.children.push(child)
    }

    pub fn build(self) -> IonStruct {
        self.children.into()
    }
}

pub struct IonListBuilder {
    children: RcRecycled<Vec<IonDomValue>>,
}

impl IonListBuilder {
    #[inline]
    pub fn add_child(&mut self, child: IonDomValue) {
        self.children.push(child)
    }

    pub fn build(self) -> IonList {
        self.children.into()
    }
}

//TODO: List, SExpression
