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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_transaction_payload(hash_byte: u8, amount: u64, memo: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&[hash_byte; 32]);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(memo);
        payload
    }

    #[test]
    fn decode_transaction_parses_amount_and_memo() {
        let payload = build_transaction_payload(0xAB, 1001, b"above");

        let tx = decode_transaction(&payload).expect("transaction should decode");

        assert_eq!(tx.amount, 1001);
        assert_eq!(tx.memo, "above");
        assert_eq!(tx.tx_hash, "abababababababababababababababababababababababababababababababab");
    }

    #[test]
    fn decode_transaction_preserves_utf8_memo() {
        let payload = build_transaction_payload(0x11, 50000, "café naïve résumé".as_bytes());

        let tx = decode_transaction(&payload).expect("utf-8 memo should decode");

        assert_eq!(tx.amount, 50000);
        assert_eq!(tx.memo, "café naïve résumé");
    }

    #[test]
    fn decode_transaction_supports_empty_memo() {
        let payload = build_transaction_payload(0x22, 2000, b"");

        let tx = decode_transaction(&payload).expect("empty memo should decode");

        assert_eq!(tx.amount, 2000);
        assert_eq!(tx.memo, "");
    }

    #[test]
    fn decode_transaction_rejects_short_payload() {
        let payload = vec![0u8; 39];

        let result = decode_transaction(&payload);

        match result {
            Err(DecodeError::PayloadTooShort { payload_len }) => {
                assert_eq!(payload_len, 39);
            }
            other => panic!("expected PayloadTooShort, got {:?}", other),
        }
    }

    #[test]
    fn decode_transaction_rejects_invalid_utf8_memo() {
        let invalid_utf8 = [0xFF, 0xFE, 0xFD];
        let payload = build_transaction_payload(0x33, 3000, &invalid_utf8);

        let result = decode_transaction(&payload);

        match result {
            Err(DecodeError::InvalidUtf8Memo) => {}
            other => panic!("expected InvalidUtf8Memo, got {:?}", other),
        }
    }

    #[test]
    fn decode_transaction_parses_big_endian_amount_correctly() {
        let amount = 0x0000_0000_0000_03E9u64; // 1001 decimal
        let payload = build_transaction_payload(0x44, amount, b"big-endian");

        let tx = decode_transaction(&payload).expect("transaction should decode");

        assert_eq!(tx.amount, 1001);
        assert_eq!(tx.memo, "big-endian");
    }
}