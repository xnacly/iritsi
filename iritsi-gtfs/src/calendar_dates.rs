use crate::err::GtfsErr;
use csv::ByteRecord;
use std::{io::Read, str};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceException {
    Added,
    Removed,
}
#[derive(Debug, PartialEq)]
pub struct CalendarDate {
    pub service_id: String,
    pub date: u32,
    pub exception: ServiceException,
}
pub struct CalendarDates<R> {
    reader: csv::Reader<R>,
    record: ByteRecord,
}
impl<R> CalendarDates<R> {
    pub(crate) fn new(reader: csv::Reader<R>) -> Self {
        Self {
            reader,
            record: ByteRecord::new(),
        }
    }
}
impl<R: Read> Iterator for CalendarDates<R> {
    type Item = Result<CalendarDate, GtfsErr>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_byte_record(&mut self.record) {
            Ok(false) => None,
            Err(error) => Some(Err(error.into())),
            Ok(true) => Some((|| {
                let field = |idx| -> Result<&str, GtfsErr> {
                    str::from_utf8(
                        self.record
                            .get(idx)
                            .ok_or(GtfsErr::GtfsFormatErr("Missing calendar_dates field"))?,
                    )
                    .map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
                };
                let exception = match field(2)?.trim() {
                    "1" => ServiceException::Added,
                    "2" => ServiceException::Removed,
                    _ => return Err(GtfsErr::GtfsFormatErr("Invalid calendar exception")),
                };
                Ok(CalendarDate {
                    service_id: field(0)?.trim().to_owned(),
                    date: field(1)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid calendar date"))?,
                    exception,
                })
            })()),
        }
    }
}
