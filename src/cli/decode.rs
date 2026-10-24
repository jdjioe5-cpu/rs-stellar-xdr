use std::ffi::OsString;
use std::io::{stdout, Write};
use std::{fmt::Debug, str::FromStr};

use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::cli::util;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("unknown type {0}, choose one of {1:?}")]
    UnknownType(String, &'static [&'static str]),
    #[error("cannot guess type, please specify --type explicitly")]
    CannotGuessType,
    #[error("could not unambiguously guess type, candidates: {0:?}")]
    AmbiguousType(Vec<&'static str>),
    #[error("error decoding XDR: {0}")]
    ReadXdr(#[from] crate::Error),
    #[error("error reading file: {0}")]
    ReadFile(std::io::Error),
    #[error("error writing output: {0}")]
    WriteOutput(std::io::Error),
    #[error("error generating JSON: {0}")]
    GenerateJson(#[from] serde_json::Error),
    #[error("type doesn't have a text representation, use 'json' as output")]
    TextUnsupported,
}

#[derive(Args, Debug, Clone)]
#[command()]
pub struct Cmd {
    /// XDR or files containing XDR to decode, or stdin if empty
    #[arg()]
    pub input: Vec<OsString>,

    /// XDR type to decode (if omitted, attempts to guess)
    #[arg(long)]
    pub r#type: Option<String>,

    // Input format of the XDR
    #[arg(long = "input", value_enum, default_value_t)]
    pub input_format: InputFormat,

    // Output format
    #[arg(long = "output", value_enum, default_value_t)]
    pub output_format: OutputFormat,

    /// Certainty as an arbitrary value when guessing the type
    #[arg(long, default_value = "2")]
    pub certainty: usize,
}

#[derive(Default, Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
pub enum InputFormat {
    Single,
    SingleBase64,
    Stream,
    #[default]
    StreamBase64,
    StreamFramed,
}

impl From<InputFormat> for crate::cli::guess::InputFormat {
    fn from(f: InputFormat) -> Self {
        match f {
            InputFormat::Single => crate::cli::guess::InputFormat::Single,
            InputFormat::SingleBase64 => crate::cli::guess::InputFormat::SingleBase64,
            InputFormat::Stream => crate::cli::guess::InputFormat::Stream,
            InputFormat::StreamBase64 => crate::cli::guess::InputFormat::StreamBase64,
            InputFormat::StreamFramed => crate::cli::guess::InputFormat::StreamFramed,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Json,
    JsonFormatted,
    Text,
    RustDebug,
    RustDebugFormatted,
}

// TODO: Remove run_x macro, it exists only to reduce the diff from when curr/next
// channels existed and each had their own run_curr/run_next invocation.
macro_rules! run_x {
    ($f:ident) => {
        fn $f(&self) -> Result<(), Error> {
            let mut input = util::parse_input(&self.input).map_err(Error::ReadFile)?;
            let explicit_type = if let Some(type_str) = &self.r#type {
                Some(crate::TypeVariant::from_str(type_str).map_err(|_| {
                    Error::UnknownType(type_str.clone(), &crate::TypeVariant::VARIANTS_STR)
                })?)
            } else {
                None
            };

            for f in &mut input {
                let mut rr = util::ResetRead::new(f);
                let r#type = if let Some(t) = explicit_type {
                    t
                } else {
                    let guessed = crate::cli::guess::guess_types(
                        &mut rr,
                        self.input_format.into(),
                        self.certainty,
                    );
                    if guessed.is_empty() {
                        return Err(Error::CannotGuessType);
                    }
                    if guessed.len() > 1 {
                        return Err(Error::AmbiguousType(
                            guessed.into_iter().map(|v| v.name()).collect(),
                        ));
                    }
                    rr.reset();
                    guessed[0]
                };

                match self.input_format {
                    InputFormat::Single => {
                        let mut l = crate::Limited::new(&mut rr, crate::Limits::none());
                        let t = crate::Type::read_xdr_to_end(r#type, &mut l)?;
                        self.out(&t)?;
                    }
                    InputFormat::SingleBase64 => {
                        let mut l = crate::Limited::new(&mut rr, crate::Limits::none());
                        let t = crate::Type::read_xdr_base64_to_end(r#type, &mut l)?;
                        self.out(&t)?;
                    }
                    InputFormat::Stream => {
                        let mut l = crate::Limited::new(&mut rr, crate::Limits::none());
                        for t in crate::Type::read_xdr_iter(r#type, &mut l) {
                            self.out(&t?)?;
                        }
                    }
                    InputFormat::StreamBase64 => {
                        let mut l = crate::Limited::new(&mut rr, crate::Limits::none());
                        for t in crate::Type::read_xdr_base64_iter(r#type, &mut l) {
                            self.out(&t?)?;
                        }
                    }
                    InputFormat::StreamFramed => {
                        let mut l = crate::Limited::new(&mut rr, crate::Limits::none());
                        for t in crate::Type::read_xdr_framed_iter(r#type, &mut l) {
                            self.out(&t?)?;
                        }
                    }
                };
            }
            Ok(())
        }
    };
}

impl Cmd {
    /// Run the CLIs decode command.
    ///
    /// ## Errors
    ///
    /// If the command is configured with state that is invalid.
    pub fn run(&self) -> Result<(), Error> {
        let result = self.run_inner();
        match result {
            Ok(()) => Ok(()),
            Err(Error::WriteOutput(e)) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
            Err(e) => Err(e),
        }
    }

    run_x!(run_inner);

    fn out(&self, v: &(impl Serialize + Debug)) -> Result<(), Error> {
        let text = match self.output_format {
            OutputFormat::Json => serde_json::to_string(v)?,
            OutputFormat::JsonFormatted => serde_json::to_string_pretty(v)?,
            OutputFormat::Text => {
                let v = serde_json::to_value(v)?;
                util::serde_json_value_to_text(v).ok_or(Error::TextUnsupported)?
            }
            OutputFormat::RustDebug => format!("{v:?}"),
            OutputFormat::RustDebugFormatted => format!("{v:#?}"),
        };
        writeln!(stdout(), "{text}").map_err(Error::WriteOutput)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::ffi::OsString;

    use super::{Cmd, Error, InputFormat, OutputFormat};

    #[test]
    fn test_decode_with_explicit_type() {
        let b64 = "AAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAABAAAAB1N0ZWxsYXIAAAAAAAAAAAAAAAAA";
        let cmd = Cmd {
            input: vec![OsString::from(b64)],
            r#type: Some("TransactionEnvelope".to_string()),
            input_format: InputFormat::SingleBase64,
            output_format: OutputFormat::Json,
            certainty: 2,
        };
        if let Err(e) = cmd.run() {
            panic!("cmd.run failed with: {e:?}");
        }
    }

    #[test]
    fn test_decode_auto_guess_single_match() {
        use crate::WriteXdr;
        let hello = crate::Hello::default();
        let b64 = hello.to_xdr_base64(crate::Limits::none()).unwrap();
        let cmd = Cmd {
            input: vec![OsString::from(b64)],
            r#type: None,
            input_format: InputFormat::SingleBase64,
            output_format: OutputFormat::Json,
            certainty: 2,
        };
        if let Err(e) = cmd.run() {
            panic!("cmd.run failed with: {e:?}");
        }
    }

    #[test]
    fn test_decode_auto_guess_cannot_guess() {
        let invalid = "not_valid_xdr_base64_data!!!";
        let cmd = Cmd {
            input: vec![OsString::from(invalid)],
            r#type: None,
            input_format: InputFormat::SingleBase64,
            output_format: OutputFormat::Json,
            certainty: 2,
        };
        let err = cmd.run().unwrap_err();
        assert!(matches!(err, Error::CannotGuessType));
    }

    #[test]
    fn test_decode_auto_guess_ambiguous() {
        // 4 zero bytes in base64: "AAAAAA=="
        // Many types can decode from 4 zero bytes (e.g. u32, bool, empty vec, int32, etc.)
        let ambiguous = "AAAAAA==";
        let cmd = Cmd {
            input: vec![OsString::from(ambiguous)],
            r#type: None,
            input_format: InputFormat::SingleBase64,
            output_format: OutputFormat::Json,
            certainty: 2,
        };
        let err = cmd.run().unwrap_err();
        match err {
            Error::AmbiguousType(candidates) => {
                assert!(candidates.len() > 1);
            }
            other => panic!("Expected AmbiguousType error, got: {other:?}"),
        }
    }
}
