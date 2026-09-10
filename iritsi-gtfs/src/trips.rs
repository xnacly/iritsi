use std::{io::Read, str};

use csv::ByteRecord;

use crate::err::GtfsErr;

#[derive(Debug, PartialEq)]
pub struct Trip {
    pub id: u32,
    pub route_id: String,
    pub service_id: String,
    pub headsign: String,
    pub direction_id: u8,
    pub shape_id: String,
}

pub struct Trips<R> {
    reader: csv::Reader<R>,
    record: ByteRecord,
}
impl<R> Trips<R> {
    pub(crate) fn new(reader: csv::Reader<R>) -> Self {
        Self {
            reader,
            record: ByteRecord::new(),
        }
    }
}

impl<R: Read> Iterator for Trips<R> {
    type Item = Result<Trip, GtfsErr>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_byte_record(&mut self.record) {
            Ok(false) => None,
            Err(error) => Some(Err(error.into())),
            Ok(true) => Some((|| {
                let field = |idx| -> Result<&str, GtfsErr> {
                    str::from_utf8(
                        self.record
                            .get(idx)
                            .ok_or(GtfsErr::GtfsFormatErr("Missing trips field"))?,
                    )
                    .map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
                };
                Ok(Trip {
                    id: field(0)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid trip id"))?,
                    route_id: field(1)?.trim().to_owned(),
                    service_id: field(2)?.trim().to_owned(),
                    headsign: field(3)?.trim().to_owned(),
                    direction_id: field(4)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid direction id"))?,
                    shape_id: field(5)?.trim().to_owned(),
                })
            })()),
        }
    }
}
