//! 1.8 entity metadata packs serializer and index into one byte; 0x7f ends it.
use super::slot::Slot;
use crate::{Error, Result, buf::Reader};
#[derive(Debug, PartialEq)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Item(Option<Slot>),
    Position([i32; 3]),
    Rotation([f32; 3]),
}
pub fn read(r: &mut Reader<'_>) -> Result<Vec<(u8, Value)>> {
    let mut result = Vec::new();
    let mut seen = 0u32;
    loop {
        let header = r.u8()?;
        if header == 0x7f {
            return Ok(result);
        }
        let index = header & 31;
        if seen & (1u32 << index) != 0 {
            return Err(Error::new("duplicate legacy metadata index"));
        }
        seen |= 1u32 << index;
        let value = match header >> 5 {
            0 => Value::Byte(r.i8()?),
            1 => Value::Short(r.i16()?),
            2 => Value::Int(r.i32()?),
            3 => Value::Float(r.f32()?),
            4 => Value::String(r.string(32767)?),
            5 => Value::Item(Slot::read(r)?),
            6 => Value::Position([r.i32()?, r.i32()?, r.i32()?]),
            7 => Value::Rotation([r.f32()?, r.f32()?, r.f32()?]),
            _ => unreachable!(),
        };
        result.push((index, value));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_header_and_terminator_are_not_modern_metadata() {
        let mut r = Reader::new(&[0, 2, 0x21, 1, 44, 0x66, 0x41, 0xa0, 0, 0, 0x7f, 99]);
        assert_eq!(
            read(&mut r).unwrap(),
            vec![
                (0, Value::Byte(2)),
                (1, Value::Short(300)),
                (6, Value::Float(20.))
            ]
        );
        assert_eq!(r.u8().unwrap(), 99);
        assert!(read(&mut Reader::new(&[0, 1, 0, 2, 0x7f])).is_err());
        assert!(read(&mut Reader::new(&[0, 1])).is_err());
    }
}
