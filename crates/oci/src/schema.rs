use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct OciSchema<T: Clone + Debug> {
    raw: Vec<u8>,
    item: T,
}

impl<T: Clone + Debug> OciSchema<T> {
    pub fn new(raw: Vec<u8>, item: T) -> OciSchema<T> {
        OciSchema { raw, item }
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn item(&self) -> &T {
        &self.item
    }

    pub fn into_raw(self) -> Vec<u8> {
        self.raw
    }

    pub fn into_item(self) -> T {
        self.item
    }

    pub fn split(self) -> (T, Vec<u8>) {
        (self.item, self.raw)
    }

    pub fn erase<X: Clone + Debug>(self) -> OciSchema<Option<X>> {
        OciSchema::new(self.into_raw(), None)
    }
}

impl<T: Clone + Debug> Into<OciSchema<Option<T>>> for OciSchema<T> {
    fn into(self) -> OciSchema<Option<T>> {
        let (item, data) = self.split();
        OciSchema::new(data, Some(item))
    }
}
