//! Fuzz target for `tpt-e-swarm-sync` message parsing and state machine.
//!
//! Run with: `cargo fuzz run fuzz_mesh_node`

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_e_swarm_sync::mesh::{Message, MessageType, MeshNode};
use tpt_e_swarm_sync::state_machine::Event;

fuzz_target!(|data: &[u8]| {
    // Interpret fuzz bytes as a sequence of messages (265 bytes each:
    // 8 sequence + 1 msg_type + 256 payload).
    if data.len() < 265 {
        return;
    }

    let mut node = MeshNode::new(1);

    for chunk in data.chunks(265) {
        let sequence = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3],
            chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
        let msg_type = MessageType::from_byte(chunk[8]).unwrap_or(MessageType::Heartbeat);
        let mut payload = [0u8; 256];
        let copy_len = (chunk.len() - 9).min(256);
        payload[..copy_len].copy_from_slice(&chunk[9..9 + copy_len]);

        let msg = Message { sequence, msg_type, payload };
        let _ = node.receive_message(msg);

        while let Some(_event) = node.process_inbound() {
            // Process all queued messages
        }

        // Verify the state machine stays consistent after each message
        assert!(node.is_consistent(), "state machine became inconsistent");
    }
});
