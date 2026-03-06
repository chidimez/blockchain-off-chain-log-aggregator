use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FilteredTransaction {
    pub tx_hash: String,
    pub amount: u64,
    pub memo: String,
}