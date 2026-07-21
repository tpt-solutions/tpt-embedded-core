//! Mesh network message types and sequencing.

/// A message exchanged between mesh nodes.
#[derive(Debug)]
pub struct Message {
    /// Monotonically increasing sequence number.
    pub sequence: u64,
    /// Message payload bytes.
    pub payload: [u8; 256],
}
