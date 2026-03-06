#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Transaction,
    StateUpdate,
    KeepAlive,
    Unknown(u8),
}

impl PacketType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0x01 => Self::Transaction,
            0x02 => Self::StateUpdate,
            0xFF => Self::KeepAlive,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::Transaction => 0x01,
            Self::StateUpdate => 0x02,
            Self::KeepAlive => 0xFF,
            Self::Unknown(value) => value,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Transaction => "transaction",
            Self::StateUpdate => "state_update",
            Self::KeepAlive => "keep_alive",
            Self::Unknown(_) => "unknown",
        }
    }
}

pub const MAGIC_BYTE: u8 = 0xA5;
pub const TX_HASH_LEN: usize = 32;
pub const TX_AMOUNT_LEN: usize = 8;
pub const TX_MIN_PAYLOAD_LEN: usize = TX_HASH_LEN + TX_AMOUNT_LEN;
pub const STATE_UPDATE_PAYLOAD_LEN: usize = 33;
pub const HEADER_LEN: usize = 4;
pub const CHECKSUM_LEN: usize = 1;