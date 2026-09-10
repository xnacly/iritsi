use crate::err::GtfsErr;
use csv::ByteRecord;
use std::{io::Read, str};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, PartialEq)]
pub struct Calendar {
    pub service_id: String,
    pub weekdays: [bool; 7],
    pub start_date: u32,
    pub end_date: u32,
}
impl Calendar {
    pub fn runs_on(&self, day: Weekday) -> bool {
        self.weekdays[day as usize]
    }
}

pub struct Calendars<R> {
    reader: csv::Reader<R>,
    record: ByteRecord,
}
impl<R> Calendars<R> {
    pub(crate) fn new(reader: csv::Reader<R>) -> Self {
        Self {
            reader,
            record: ByteRecord::new(),
        }
    }
}

impl<R: Read> Iterator for Calendars<R> {
    type Item = Result<Calendar, GtfsErr>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_byte_record(&mut self.record) {
            Ok(false) => None,
            Err(error) => Some(Err(error.into())),
            Ok(true) => Some((|| {
                let field = |idx| -> Result<&str, GtfsErr> {
                    str::from_utf8(
                        self.record
                            .get(idx)
                            .ok_or(GtfsErr::GtfsFormatErr("Missing calendar field"))?,
                    )
                    .map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
                };
                let active = |idx| -> Result<bool, GtfsErr> {
                    match field(idx)?.trim() {
                        "0" => Ok(false),
                        "1" => Ok(true),
                        _ => Err(GtfsErr::GtfsFormatErr("Invalid calendar weekday")),
                    }
                };
                Ok(Calendar {
                    service_id: field(0)?.trim().to_owned(),
                    weekdays: [
                        active(1)?,
                        active(2)?,
                        active(3)?,
                        active(4)?,
                        active(5)?,
                        active(6)?,
                        active(7)?,
                    ],
                    start_date: field(8)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid calendar start date"))?,
                    end_date: field(9)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid calendar end date"))?,
                })
            })()),
        }
    }
}
