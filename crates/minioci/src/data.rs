use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct OciStruct<T: Clone + Debug> {
    raw: Vec<u8>,
    item: T,
}

impl<T: Clone + Debug> OciStruct<T> {
    pub fn new(raw: Vec<u8>, item: T) -> OciStruct<T> {
        OciStruct { raw, item }
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

    pub fn erase<X: Clone + Debug>(self) -> OciStruct<Option<X>> {
        OciStruct::new(self.into_raw(), None)
    }
}

impl<T: Clone + Debug> Into<OciStruct<Option<T>>> for OciStruct<T> {
    fn into(self) -> OciStruct<Option<T>> {
        let (item, data) = self.split();
        OciStruct::new(data, Some(item))
    }
}
