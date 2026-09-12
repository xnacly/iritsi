///! file system abstraction for atomic storage updates
mod fs;
///! sqlite backed data store engine
mod store;

pub struct Iritsi {
    store: store::Store,
}

impl Iritsi {
    pub fn ingest() {}
}
