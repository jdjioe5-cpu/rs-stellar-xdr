use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, Cursor, Error as IoError, Read};
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

/// Reads a stream and yields separate base64 encoded strings.
/// Separates base64 entities by whitespace, or across '=' padding boundaries.
pub struct SeparateBase64Tokens<R: Read> {
    reader: R,
    buf: [u8; 8192],
    pos: usize,
    cap: usize,
    current: String,
    in_padding: bool,
    eof: bool,
}

impl<R: Read> SeparateBase64Tokens<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buf: [0u8; 8192],
            pos: 0,
            cap: 0,
            current: String::new(),
            in_padding: false,
            eof: false,
        }
    }
}

impl<R: Read> Iterator for SeparateBase64Tokens<R> {
    type Item = Result<String, std::io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pos >= self.cap {
                if self.eof {
                    if !self.current.is_empty() {
                        let token = std::mem::take(&mut self.current);
                        self.in_padding = false;
                        return Some(Ok(token));
                    }
                    return None;
                }
                match self.reader.read(&mut self.buf) {
                    Ok(0) => {
                        self.eof = true;
                        if !self.current.is_empty() {
                            let token = std::mem::take(&mut self.current);
                            self.in_padding = false;
                            return Some(Ok(token));
                        }
                        return None;
                    }
                    Ok(n) => {
                        self.pos = 0;
                        self.cap = n;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Some(Err(e)),
                }
            }

            let b = self.buf[self.pos];
            self.pos += 1;

            if b.is_ascii_whitespace() {
                if !self.current.is_empty() {
                    let token = std::mem::take(&mut self.current);
                    self.in_padding = false;
                    return Some(Ok(token));
                }
            } else if b == b'=' {
                self.in_padding = true;
                self.current.push('=');
            } else if b.is_ascii_alphanumeric() || b == b'+' || b == b'/' {
                if self.in_padding {
                    // A new base64 entity starts after padding of the previous one
                    let token = std::mem::take(&mut self.current);
                    self.current.push(b as char);
                    self.in_padding = false;
                    return Some(Ok(token));
                }
                self.current.push(b as char);
            } else {
                self.current.push(b as char);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_separate_base64_tokens_newlines() {
        let input = "AAAABAAAAAE=\nAAAABAAAAAI=\n";
        let tokens: Vec<String> = SeparateBase64Tokens::new(input.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tokens, vec!["AAAABAAAAAE=", "AAAABAAAAAI="]);
    }

    #[test]
    fn test_separate_base64_tokens_spaces_and_tabs() {
        let input = "  AAAABAAAAAE= \t AAAABAAAAAI=  ";
        let tokens: Vec<String> = SeparateBase64Tokens::new(input.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tokens, vec!["AAAABAAAAAE=", "AAAABAAAAAI="]);
    }

    #[test]
    fn test_separate_base64_tokens_tightly_packed_padding() {
        let input = "AAAABAAAAAE=AAAABAAAAAI=";
        let tokens: Vec<String> = SeparateBase64Tokens::new(input.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tokens, vec!["AAAABAAAAAE=", "AAAABAAAAAI="]);
    }

    #[test]
    fn test_separate_base64_tokens_double_padding_tightly_packed() {
        let input = "AAAAAA==AAAAAQ==";
        let tokens: Vec<String> = SeparateBase64Tokens::new(input.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tokens, vec!["AAAAAA==", "AAAAAQ=="]);
    }

    #[test]
    fn test_separate_base64_tokens_empty_and_whitespace() {
        let empty = "";
        let tokens: Vec<String> = SeparateBase64Tokens::new(empty.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert!(tokens.is_empty());

        let spaces = "  \n\t \r\n ";
        let tokens: Vec<String> = SeparateBase64Tokens::new(spaces.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_separate_base64_tokens_single_no_newline() {
        let input = "AAAABAAAAAE=";
        let tokens: Vec<String> = SeparateBase64Tokens::new(input.as_bytes())
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(tokens, vec!["AAAABAAAAAE="]);
    }
}
