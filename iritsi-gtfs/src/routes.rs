use std::{io::Read, str};

use csv::ByteRecord;

use crate::err::GtfsErr;

#[derive(Debug, PartialEq)]
pub struct Route {
    pub id: String,
    pub short_name: String,
    pub long_name: String,
    pub route_type: u16,
    pub color: String,
}

pub struct Routes<R> {
    reader: csv::Reader<R>,
    record: ByteRecord,
}

impl<R> Routes<R> {
    pub(crate) fn new(reader: csv::Reader<R>) -> Self {
        Self {
            reader,
            record: ByteRecord::new(),
        }
    }
}

impl<R: Read> Iterator for Routes<R> {
    type Item = Result<Route, GtfsErr>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_byte_record(&mut self.record) {
            Ok(false) => None,
            Err(error) => Some(Err(error.into())),
            Ok(true) => Some((|| {
                let field = |idx| -> Result<&str, GtfsErr> {
                    str::from_utf8(
                        self.record
                            .get(idx)
                            .ok_or(GtfsErr::GtfsFormatErr("Missing routes field"))?,
                    )
                    .map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
                };
                Ok(Route {
                    id: field(0)?.trim().to_owned(),
                    short_name: field(1)?.trim().to_owned(),
                    long_name: field(2)?.trim().to_owned(),
                    route_type: field(3)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid route type"))?,
                    color: field(4)?.trim().to_owned(),
                })
            })()),
        }
    }
}
