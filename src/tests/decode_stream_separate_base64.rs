use std::ffi::OsString;

use crate::cli::decode::{Cmd as DecodeCmd, InputFormat as DecodeInputFormat, OutputFormat};
use crate::cli::guess::{Cmd as GuessCmd, InputFormat as GuessInputFormat};

#[test]
fn test_decode_separate_base64_scval_stream() {
    // Two ScVal entities: {"i32": 1} and {"i32": 2}
    // Encoded as: AAAABAAAAAE= and AAAABAAAAAI=
    let input = "AAAABAAAAAE=\nAAAABAAAAAI=\n";
    let cmd = DecodeCmd {
        input: vec![OsString::from(input)],
        r#type: "ScVal".to_string(),
        input_format: DecodeInputFormat::StreamSeparateBase64,
        output_format: OutputFormat::Json,
    };
    assert!(cmd.run().is_ok());
}

#[test]
fn test_decode_separate_base64_multiline_and_spaces() {
    // Spaces between separate entities
    let input = "AAAABAAAAAE=    AAAABAAAAAI=";
    let cmd = DecodeCmd {
        input: vec![OsString::from(input)],
        r#type: "ScVal".to_string(),
        input_format: DecodeInputFormat::StreamSeparateBase64,
        output_format: OutputFormat::Json,
    };
    assert!(cmd.run().is_ok());
}

#[test]
fn test_decode_separate_base64_concatenated_padding() {
    // Two base64 entities directly concatenated without whitespace
    let input = "AAAABAAAAAE=AAAABAAAAAI=";
    let cmd = DecodeCmd {
        input: vec![OsString::from(input)],
        r#type: "ScVal".to_string(),
        input_format: DecodeInputFormat::StreamSeparateBase64,
        output_format: OutputFormat::Json,
    };
    assert!(cmd.run().is_ok());
}

#[test]
fn test_guess_separate_base64_stream() {
    let input = "AAAABAAAAAE=\nAAAABAAAAAI=\n";
    let cmd = GuessCmd {
        input: Some(OsString::from(input)),
        input_format: GuessInputFormat::StreamSeparateBase64,
        output_format: crate::cli::guess::OutputFormat::List,
        certainty: 2,
    };
    assert!(cmd.run().is_ok());
}
