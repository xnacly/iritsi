use crate::err::GtfsErr;
use csv::ByteRecord;
use std::{io::Read, str};

#[derive(Debug, PartialEq)]
pub struct ShapePoint {
    pub shape_id: String,
    pub lat: f32,
    pub lon: f32,
    pub sequence: u32,
    pub distance_traveled: f32,
}
pub struct Shapes<R> {
    reader: csv::Reader<R>,
    record: ByteRecord,
}
impl<R> Shapes<R> {
    pub(crate) fn new(reader: csv::Reader<R>) -> Self {
        Self {
            reader,
            record: ByteRecord::new(),
        }
    }
}
impl<R: Read> Iterator for Shapes<R> {
    type Item = Result<ShapePoint, GtfsErr>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_byte_record(&mut self.record) {
            Ok(false) => None,
            Err(error) => Some(Err(error.into())),
            Ok(true) => Some((|| {
                let field = |idx| -> Result<&str, GtfsErr> {
                    str::from_utf8(
                        self.record
                            .get(idx)
                            .ok_or(GtfsErr::GtfsFormatErr("Missing shapes field"))?,
                    )
                    .map_err(|_| GtfsErr::GtfsFormatErr("Invalid UTF-8"))
                };
                Ok(ShapePoint {
                    shape_id: field(0)?.trim().to_owned(),
                    lat: field(1)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid shape latitude"))?,
                    lon: field(2)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid shape longitude"))?,
                    sequence: field(3)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid shape sequence"))?,
                    distance_traveled: field(4)?
                        .trim()
                        .parse()
                        .map_err(|_| GtfsErr::GtfsFormatErr("Invalid shape distance"))?,
                })
            })()),
        }
    }
}
