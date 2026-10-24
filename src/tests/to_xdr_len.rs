#![cfg(feature = "std")]

use crate::{
    Asset, AssetCode4, BytesM, Error, Limits, Memo, PublicKey, PublicKeyType, StringM, Uint256,
    VecM, WriteXdr,
};

#[test]
fn test_primitive_types_to_xdr_len() -> Result<(), Error> {
    let limits = Limits::none();

    // i32: 4 bytes
    let val_i32 = 42i32;
    assert_eq!(val_i32.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        val_i32.to_xdr_len(limits.clone())?,
        val_i32.to_xdr(limits.clone())?.len()
    );

    // u32: 4 bytes
    let val_u32 = 12345u32;
    assert_eq!(val_u32.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        val_u32.to_xdr_len(limits.clone())?,
        val_u32.to_xdr(limits.clone())?.len()
    );

    // i64: 8 bytes
    let val_i64 = -9999999999i64;
    assert_eq!(val_i64.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        val_i64.to_xdr_len(limits.clone())?,
        val_i64.to_xdr(limits.clone())?.len()
    );

    // u64: 8 bytes
    let val_u64 = 0xdeadbeefcafebabeu64;
    assert_eq!(val_u64.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        val_u64.to_xdr_len(limits.clone())?,
        val_u64.to_xdr(limits.clone())?.len()
    );

    // bool: 4 bytes in XDR
    let val_bool = true;
    assert_eq!(val_bool.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        val_bool.to_xdr_len(limits.clone())?,
        val_bool.to_xdr(limits.clone())?.len()
    );

    // Option: 4 bytes discriminant + payload if Some
    let val_none: Option<i32> = None;
    assert_eq!(val_none.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        val_none.to_xdr_len(limits.clone())?,
        val_none.to_xdr(limits.clone())?.len()
    );

    let val_some: Option<i32> = Some(100);
    assert_eq!(val_some.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        val_some.to_xdr_len(limits.clone())?,
        val_some.to_xdr(limits.clone())?.len()
    );

    Ok(())
}

#[test]
fn test_fixed_size_types_to_xdr_len() -> Result<(), Error> {
    let limits = Limits::none();

    let u256 = Uint256([0x7f; 32]);
    let len = u256.to_xdr_len(limits.clone())?;
    assert_eq!(len, 32);
    assert_eq!(len, u256.to_xdr(limits.clone())?.len());

    let pk = PublicKey::PublicKeyTypeEd25519(u256);
    let pk_len = pk.to_xdr_len(limits.clone())?;
    assert_eq!(pk_len, 4 + 32);
    assert_eq!(pk_len, pk.to_xdr(limits.clone())?.len());

    Ok(())
}

#[test]
fn test_variable_length_types_with_padding_to_xdr_len() -> Result<(), Error> {
    let limits = Limits::none();

    // 1-byte StringM: 4 byte length header + 1 byte data + 3 bytes zero padding = 8 bytes
    let s1: StringM<64> = "a".try_into()?;
    assert_eq!(s1.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        s1.to_xdr_len(limits.clone())?,
        s1.to_xdr(limits.clone())?.len()
    );

    // 2-byte StringM: 4 + 2 + 2 = 8 bytes
    let s2: StringM<64> = "ab".try_into()?;
    assert_eq!(s2.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        s2.to_xdr_len(limits.clone())?,
        s2.to_xdr(limits.clone())?.len()
    );

    // 3-byte StringM: 4 + 3 + 1 = 8 bytes
    let s3: StringM<64> = "abc".try_into()?;
    assert_eq!(s3.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        s3.to_xdr_len(limits.clone())?,
        s3.to_xdr(limits.clone())?.len()
    );

    // 4-byte StringM: 4 + 4 + 0 = 8 bytes
    let s4: StringM<64> = "abcd".try_into()?;
    assert_eq!(s4.to_xdr_len(limits.clone())?, 8);
    assert_eq!(
        s4.to_xdr_len(limits.clone())?,
        s4.to_xdr(limits.clone())?.len()
    );

    // 5-byte StringM: 4 + 5 + 3 = 12 bytes
    let s5: StringM<64> = "abcde".try_into()?;
    assert_eq!(s5.to_xdr_len(limits.clone())?, 12);
    assert_eq!(
        s5.to_xdr_len(limits.clone())?,
        s5.to_xdr(limits.clone())?.len()
    );

    // BytesM: same padding rules
    let b: BytesM<64> = vec![1, 2, 3, 4, 5, 6, 7].try_into()?;
    let b_len = b.to_xdr_len(limits.clone())?;
    assert_eq!(b_len, 4 + 7 + 1); // 12 bytes
    assert_eq!(b_len, b.to_xdr(limits.clone())?.len());

    // VecM of i32: 4 byte length header + 3 * 4 byte elements = 16 bytes
    let v: VecM<i32, 10> = vec![10, 20, 30].try_into()?;
    let v_len = v.to_xdr_len(limits.clone())?;
    assert_eq!(v_len, 4 + 12);
    assert_eq!(v_len, v.to_xdr(limits.clone())?.len());

    Ok(())
}

#[test]
fn test_unions_and_enums_to_xdr_len() -> Result<(), Error> {
    let limits = Limits::none();

    // Enum: 4 bytes
    let pk_type = PublicKeyType::PublicKeyTypeEd25519;
    assert_eq!(pk_type.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        pk_type.to_xdr_len(limits.clone())?,
        pk_type.to_xdr(limits.clone())?.len()
    );

    // Union variants:
    // Memo::None: discriminant 0 (4 bytes)
    let memo_none = Memo::None;
    assert_eq!(memo_none.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        memo_none.to_xdr_len(limits.clone())?,
        memo_none.to_xdr(limits.clone())?.len()
    );

    // Memo::Id: discriminant 4 (4 bytes) + u64 (8 bytes) = 12 bytes
    let memo_id = Memo::Id(123456789);
    assert_eq!(memo_id.to_xdr_len(limits.clone())?, 12);
    assert_eq!(
        memo_id.to_xdr_len(limits.clone())?,
        memo_id.to_xdr(limits.clone())?.len()
    );

    // Memo::Text: discriminant 1 (4 bytes) + StringM<28> (4 + text_len + pad)
    let memo_text = Memo::Text("stellar".try_into()?);
    let expected_len = 4 + (4 + 7 + 1); // 16 bytes
    assert_eq!(memo_text.to_xdr_len(limits.clone())?, expected_len);
    assert_eq!(
        memo_text.to_xdr_len(limits.clone())?,
        memo_text.to_xdr(limits.clone())?.len()
    );

    // Asset::Native: discriminant 0 (4 bytes)
    let asset_native = Asset::Native;
    assert_eq!(asset_native.to_xdr_len(limits.clone())?, 4);
    assert_eq!(
        asset_native.to_xdr_len(limits.clone())?,
        asset_native.to_xdr(limits.clone())?.len()
    );

    // Asset::CreditAlphanum4: discriminant (4) + code4 (4) + issuer (4 + 32) = 44 bytes
    let asset_alpha4 = Asset::CreditAlphanum4(crate::AlphaNum4 {
        asset_code: AssetCode4([b'U', b'S', b'D', 0]),
        issuer: crate::AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([1; 32]))),
    });
    let a4_len = asset_alpha4.to_xdr_len(limits.clone())?;
    assert_eq!(a4_len, 44);
    assert_eq!(a4_len, asset_alpha4.to_xdr(limits.clone())?.len());

    Ok(())
}

#[test]
fn test_depth_limit_exceeded_in_to_xdr_len() {
    // Setting depth limit to 0 should fail both to_xdr and to_xdr_len with DepthLimitExceeded
    let mut limits = Limits::none();
    limits.depth = 0;

    let val = 42i32;
    assert!(matches!(
        val.to_xdr(limits.clone()),
        Err(Error::DepthLimitExceeded)
    ));
    assert!(matches!(
        val.to_xdr_len(limits),
        Err(Error::DepthLimitExceeded)
    ));
}
