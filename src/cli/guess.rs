use clap::{Args, ValueEnum};
use std::ffi::OsString;
use std::fs::File;
use std::io::{stdin, stdout, Cursor, Read, Write};
use std::path::Path;

use crate::cli::util::ResetRead;

#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error("error decoding XDR: {0}")]
    ReadXdr(#[from] crate::Error),
    #[error("error reading file: {0}")]
    ReadFile(std::io::Error),
    #[error("error writing output: {0}")]
    WriteOutput(std::io::Error),
}

#[derive(Args, Debug, Clone)]
#[command()]
pub struct Cmd {
    /// XDR or file containing XDR to decode, or stdin if empty
    #[arg()]
    pub input: Option<OsString>,

    // Input format
    #[arg(long = "input", value_enum, default_value_t)]
    pub input_format: InputFormat,

    // Output format
    #[arg(long = "output", value_enum, default_value_t)]
    pub output_format: OutputFormat,

    /// Certainty as an arbitrary value
    #[arg(long, default_value = "2")]
    pub certainty: usize,
}

#[derive(Default, Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
pub enum InputFormat {
    Single,
    #[default]
    SingleBase64,
    Stream,
    StreamBase64,
    StreamFramed,
}

#[derive(Default, Clone, Copy, Debug, Eq, Hash, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    List,
}

/// Guess which XDR types can decode the input stream.
pub fn guess_types<R: Read>(
    rr: &mut ResetRead<R>,
    format: InputFormat,
    certainty: usize,
) -> Vec<crate::TypeVariant> {
    let mut guessed = Vec::new();
    'variants: for v in crate::TypeVariant::VARIANTS {
        rr.reset();
        let count: usize = match format {
            InputFormat::Single => {
                let mut l = crate::Limited::new(&mut *rr, crate::Limits::none());
                crate::Type::read_xdr_to_end(v, &mut l)
                    .ok()
                    .map(|_| 1)
                    .unwrap_or_default()
            }
            InputFormat::SingleBase64 => {
                let mut l = crate::Limited::new(&mut *rr, crate::Limits::none());
                crate::Type::read_xdr_base64_to_end(v, &mut l)
                    .ok()
                    .map(|_| 1)
                    .unwrap_or_default()
            }
            InputFormat::Stream => {
                let mut l = crate::Limited::new(&mut *rr, crate::Limits::none());
                let iter = crate::Type::read_xdr_iter(v, &mut l);
                let iter = iter.take(certainty);
                let mut count = 0;
                for v in iter {
                    match v {
                        Ok(_) => count += 1,
                        Err(_) => continue 'variants,
                    }
                }
                count
            }
            InputFormat::StreamBase64 => {
                let mut l = crate::Limited::new(&mut *rr, crate::Limits::none());
                let iter = crate::Type::read_xdr_base64_iter(v, &mut l);
                let iter = iter.take(certainty);
                let mut count = 0;
                for v in iter {
                    match v {
                        Ok(_) => count += 1,
                        Err(_) => continue 'variants,
                    }
                }
                count
            }
            InputFormat::StreamFramed => {
                let mut l = crate::Limited::new(&mut *rr, crate::Limits::none());
                let iter = crate::Type::read_xdr_framed_iter(v, &mut l);
                let iter = iter.take(certainty);
                let mut count = 0;
                for v in iter {
                    match v {
                        Ok(_) => count += 1,
                        Err(_) => continue 'variants,
                    }
                }
                count
            }
        };
        if count > 0 {
            guessed.push(v);
        }
    }
    guessed
}

macro_rules! run_x {
    ($f:ident) => {
        fn $f(&self) -> Result<(), Error> {
            let mut rr = ResetRead::new(self.input()?);
            let guessed = guess_types(&mut rr, self.input_format, self.certainty);
            if guessed.is_empty() {
                std::process::exit(1);
            }
            for v in guessed {
                writeln!(stdout(), "{}", v.name()).map_err(Error::WriteOutput)?;
            }
            Ok(())
        }
    };
}

impl Cmd {
    /// Run the CLIs guess command.
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

    fn input(&self) -> Result<Box<dyn Read>, Error> {
        if let Some(input) = &self.input {
            let exist = Path::new(input).try_exists();
            if let Ok(true) = exist {
                Ok(Box::new(File::open(input).map_err(Error::ReadFile)?))
            } else {
                Ok(Box::new(Cursor::new(input.clone().into_encoded_bytes())))
            }
        } else {
            Ok(Box::new(stdin()))
        }
    }
}
