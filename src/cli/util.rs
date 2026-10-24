use std::cmp;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, stdin, Cursor, Error as IoError, Read};
use std::path::Path;

pub fn parse_input(input: &[OsString]) -> Result<Vec<Box<dyn Read>>, IoError> {
    if input.is_empty() {
        Ok(vec![Box::new(stdin())])
    } else {
        Ok(input
            .iter()
            .map(|input| {
                let exist = Path::new(input).try_exists();
                if let Ok(true) = exist {
                    let file = File::open(input)?;
                    Ok::<Box<dyn Read>, IoError>(Box::new(file) as Box<dyn Read>)
                } else {
                    Ok(Box::new(Cursor::new(input.clone().into_encoded_bytes())) as Box<dyn Read>)
                }
            })
            .collect::<Result<Vec<_>, _>>()?)
    }
}

pub fn serde_json_value_to_text(v: serde_json::Value) -> Option<String> {
    let s = match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::String(s) => s,
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None?,
    };
    Some(s)
}

pub struct ResetRead<R: Read> {
    read: R,
    buf: Vec<u8>,
    cursor: usize,
}

impl<R: Read> ResetRead<R> {
    pub fn new(r: R) -> Self {
        Self {
            read: r,
            buf: Vec::new(),
            cursor: 0,
        }
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

impl<R: Read> Read for ResetRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = cmp::min(self.buf.len() - self.cursor, buf.len());
        buf[..n].copy_from_slice(&self.buf[self.cursor..self.cursor + n]);
        if n < buf.len() {
            let read_n = self.read.read(&mut buf[n..])?;
            self.buf.extend_from_slice(&buf[n..n + read_n]);
            self.cursor += n + read_n;
            Ok(n + read_n)
        } else {
            self.cursor += n;
            Ok(n)
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        error,
        io::{Cursor, Read},
    };

    use super::ResetRead;

    #[test]
    fn test_reset_read() -> Result<(), Box<dyn error::Error>> {
        let source: Vec<u8> = (0..8).collect();
        let reader = Cursor::new(source);
        let mut rr = ResetRead::new(reader);

        let mut buf = [0u8; 4];
        let n = rr.read(&mut buf)?;
        assert_eq!(n, 4);
        assert_eq!(buf, [0, 1, 2, 3]);

        let mut buf = [0u8; 4];
        let n = rr.read(&mut buf)?;
        assert_eq!(n, 4);
        assert_eq!(buf, [4, 5, 6, 7]);

        let n = rr.read(&mut buf)?;
        assert_eq!(n, 0);

        rr.reset();
        let mut buf = [0u8; 4];
        let n = rr.read(&mut buf)?;
        assert_eq!(n, 4);
        assert_eq!(buf, [0, 1, 2, 3]);

        Ok(())
    }

    #[test]
    fn test_reset_read_partial_cache_overlap() -> Result<(), Box<dyn error::Error>> {
        let source: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        ];
        let reader = Cursor::new(source);
        let mut rr = ResetRead::new(reader);

        let mut buf5 = [0u8; 5];
        let n = rr.read(&mut buf5)?;
        assert_eq!(n, 5);
        assert_eq!(buf5, [0x00, 0x00, 0x00, 0x00, 0x01]);

        rr.reset();

        let mut buf4 = [0u8; 4];
        let n = rr.read(&mut buf4)?;
        assert_eq!(n, 4);
        assert_eq!(buf4, [0x00, 0x00, 0x00, 0x00]);

        let mut buf4 = [0u8; 4];
        let n = rr.read(&mut buf4)?;
        assert_eq!(n, 4);
        assert_eq!(buf4, [0x01, 0x02, 0x03, 0x04]);

        let mut buf3 = [0u8; 3];
        let n = rr.read(&mut buf3)?;
        assert_eq!(n, 3);
        assert_eq!(buf3, [0x05, 0x06, 0x07]);

        let mut buf1 = [0u8; 1];
        let n = rr.read(&mut buf1)?;
        assert_eq!(n, 1);
        assert_eq!(buf1, [0x08]);

        Ok(())
    }
}
