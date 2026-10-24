#![cfg(feature = "std")]

use crate::{Error, Limited, Limits, Memo, PeekRead, PublicKey, ReadXdr, Uint256, WriteXdr};
use std::io::{BufRead, BufReader, Cursor, Read};

#[test]
fn test_peek_read_empty_eof() -> Result<(), std::io::Error> {
    let cursor = Cursor::new(Vec::<u8>::new());
    let mut reader = PeekRead::new(cursor);

    // fill_buf on empty reader returns empty slice
    assert_eq!(reader.fill_buf()?, &[] as &[u8]);

    // Multiple fill_buf calls on EOF consistently return empty slice
    assert_eq!(reader.fill_buf()?, &[] as &[u8]);

    // read on empty reader returns 0
    let mut buf = [0u8; 4];
    assert_eq!(reader.read(&mut buf)?, 0);

    Ok(())
}

#[test]
fn test_peek_read_fill_buf_peek_then_read() -> Result<(), std::io::Error> {
    let data = vec![10u8, 20, 30, 40, 50];
    let cursor = Cursor::new(data);
    let mut reader = PeekRead::new(cursor);

    // First peek should yield the first byte without advancing the underlying stream
    assert_eq!(reader.fill_buf()?, &[10]);
    // Repeating fill_buf returns the same peeked byte
    assert_eq!(reader.fill_buf()?, &[10]);

    // Reading 1 byte consumes the peeked byte
    let mut b1 = [0u8; 1];
    assert_eq!(reader.read(&mut b1)?, 1);
    assert_eq!(b1[0], 10);

    // Reading into a larger buffer should now read directly from the underlying stream
    let mut rest = [0u8; 4];
    reader.read_exact(&mut rest)?;
    assert_eq!(&rest, &[20, 30, 40, 50]);

    // Now at EOF
    assert_eq!(reader.fill_buf()?, &[] as &[u8]);
    assert_eq!(reader.read(&mut rest)?, 0);

    Ok(())
}

#[test]
fn test_peek_read_consume() -> Result<(), std::io::Error> {
    let data = vec![1, 2, 3];
    let cursor = Cursor::new(data);
    let mut reader = PeekRead::new(cursor);

    // Peek first byte
    assert_eq!(reader.fill_buf()?, &[1]);

    // Consume peeked byte
    reader.consume(1);

    // Next peek should yield second byte
    assert_eq!(reader.fill_buf()?, &[2]);

    // Read remaining bytes
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    assert_eq!(buf, vec![2, 3]);

    Ok(())
}

#[test]
fn test_read_xdr_iter_primitives() -> Result<(), Error> {
    let limits = Limits::none();
    let mut encoded = Vec::new();

    let items = vec![100u32, 200u32, 300u32, 400u32];
    for &item in &items {
        encoded.extend_from_slice(&item.to_xdr(limits.clone())?);
    }

    let mut cursor = Limited::new(Cursor::new(encoded), limits.clone());
    let iter = u32::read_xdr_iter(&mut cursor);

    let decoded: Vec<u32> = iter.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(decoded, items);

    Ok(())
}

#[test]
fn test_read_xdr_iter_composite_types() -> Result<(), Error> {
    let limits = Limits::none();
    let mut encoded = Vec::new();

    let m1 = Memo::None;
    let m2 = Memo::Id(987654321);
    let m3 = Memo::Text("stellar".try_into()?);

    encoded.extend_from_slice(&m1.to_xdr(limits.clone())?);
    encoded.extend_from_slice(&m2.to_xdr(limits.clone())?);
    encoded.extend_from_slice(&m3.to_xdr(limits.clone())?);

    let mut cursor = Limited::new(Cursor::new(encoded), limits.clone());
    let mut iter = Memo::read_xdr_iter(&mut cursor);

    assert_eq!(iter.next(), Some(Ok(m1)));
    assert_eq!(iter.next(), Some(Ok(m2)));
    assert_eq!(iter.next(), Some(Ok(m3)));
    assert_eq!(iter.next(), None);

    Ok(())
}

#[test]
fn test_read_xdr_iter_empty_stream() -> Result<(), Error> {
    let mut cursor = Limited::new(Cursor::new(Vec::<u8>::new()), Limits::none());
    let mut iter = u32::read_xdr_iter(&mut cursor);

    assert_eq!(iter.next(), None);
    Ok(())
}

#[test]
fn test_read_xdr_iter_truncated_stream() -> Result<(), Error> {
    // A u32 requires 4 bytes, provide only 2 bytes
    let truncated_bytes = vec![0x00, 0x01];
    let mut cursor = Limited::new(Cursor::new(truncated_bytes), Limits::none());
    let mut iter = u32::read_xdr_iter(&mut cursor);

    match iter.next() {
        Some(Err(Error::Io(e))) => {
            assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof);
        }
        other => panic!(
            "Expected Some(Err(Error::Io(UnexpectedEof))), got {:?}",
            other
        ),
    }

    Ok(())
}

#[test]
fn test_read_xdr_iter_with_underlying_bufreader() -> Result<(), Error> {
    let limits = Limits::none();
    let items = vec![
        PublicKey::PublicKeyTypeEd25519(Uint256([1; 32])),
        PublicKey::PublicKeyTypeEd25519(Uint256([2; 32])),
    ];

    let mut encoded = Vec::new();
    for item in &items {
        encoded.extend_from_slice(&item.to_xdr(limits.clone())?);
    }

    // User already wrapped their stream in a BufReader
    let buf_reader = BufReader::new(Cursor::new(encoded));
    let mut cursor = Limited::new(buf_reader, limits.clone());
    let iter = PublicKey::read_xdr_iter(&mut cursor);

    let decoded: Vec<PublicKey> = iter.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(decoded, items);

    Ok(())
}
