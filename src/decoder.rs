#![allow(dead_code)]
use std::str;

use crate::output::FilteredTransaction;
use crate::protocol::{TX_AMOUNT_LEN, TX_HASH_LEN, TX_MIN_PAYLOAD_LEN};

#[derive(Debug)]
pub enum DecodeError {
    PayloadTooShort { payload_len: usize },
    InvalidUtf8Memo,
}

pub fn decode_transaction(payload: &[u8]) -> Result<FilteredTransaction, DecodeError> {
    if payload.len() < TX_MIN_PAYLOAD_LEN {
        return Err(DecodeError::PayloadTooShort {
            payload_len: payload.len(),
        });
    }

    let hash_bytes = &payload[..TX_HASH_LEN];

    let amount_start = TX_HASH_LEN;
    let amount_end = TX_HASH_LEN + TX_AMOUNT_LEN;

    let amount_slice = &payload[amount_start..amount_end];
    let amount_bytes: [u8; TX_AMOUNT_LEN] = match amount_slice.try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err(DecodeError::PayloadTooShort {
                payload_len: payload.len(),
            });
        }
    };

    let amount = u64::from_be_bytes(amount_bytes);

    let memo_bytes = &payload[amount_end..];
    let memo = match str::from_utf8(memo_bytes) {
        Ok(s) => s.to_string(),
        Err(_) => return Err(DecodeError::InvalidUtf8Memo),
    };

    Ok(FilteredTransaction {
        tx_hash: to_hex_lower(hash_bytes),
        amount,
        memo,
    })
}

fn to_hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}