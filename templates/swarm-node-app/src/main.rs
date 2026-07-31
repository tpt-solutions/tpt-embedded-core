//! Battery-powered mesh sensor node scaffold built on `tpt-embedded-core`.
//!
//! Demonstrates the "wake, sample, coordinate, sleep" shape a real swarm
//! node follows, wiring together:
//! - `tpt_e_swarm_sync`'s state machine — join or maintain the mesh's
//!   Primary/Secondary election each wake cycle
//! - `tpt_e_chronos::RingBuf` — buffer sensor samples ISR-safely between
//!   wake cycles
//! - `tpt_e_slumber` — the proof-token-gated deep sleep this node returns
//!   to once its work for this cycle is done
//!
//! This template uses the `mock` feature so it runs on host without
//! hardware. Swap to real esp-hal backends (a real ESP-NOW driver feeding
//! `MeshStateMachine::process_event`, a real ADC/sensor read, and
//! `use_esp_hal` on `tpt-e-slumber`) once you have a board.

#![no_std]
#![no_main]

use esp_hal::prelude::*;
use tpt_e_chronos::ring_buf::RingBuf;
use tpt_e_slumber::sleep::SleepController;
use tpt_e_slumber::tokens::{BuffersFlushedToken, DmaParkedToken, RtcIsolatedToken};
use tpt_e_swarm_sync::state_machine::{Event, MeshStateMachine, NodeRole};

/// This node's ID. Every node in the mesh must have a distinct value —
/// election ties break in favor of the *lower* ID (see
/// `tpt-e-swarm-sync`'s "deterministic tie-breaking" guarantee).
const NODE_ID: u32 = 1;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[entry]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default());

    let mut mesh = MeshStateMachine::new(NODE_ID);
    let telemetry: RingBuf<u32, 32> = RingBuf::new(0);
    let sleep_controller = SleepController::new();

    loop {
        // -------------------------------------------------------------------
        // 1. Coordinate — join or maintain the mesh's election state.
        // -------------------------------------------------------------------
        // This scaffold has no real radio driver wired up, so it always
        // takes the "no other nodes heard yet" path. On real hardware, feed
        // `mesh.process_event(...)` from your ESP-NOW/802.15.4 receive
        // handler instead: `Event::HeartbeatReceived { sender_id }` when you
        // hear another node, `Event::NoOtherNodesFound` only after a real
        // listen-before-claim timeout.
        if mesh.role() == NodeRole::Unknown {
            let transition = mesh.process_event(Event::NoOtherNodesFound);
            let _ = transition.should_broadcast; // send a heartbeat announcing this role, on real hardware
        }

        // -------------------------------------------------------------------
        // 2. Sample — read a sensor and buffer it ISR-safely.
        // -------------------------------------------------------------------
        let sample = read_sensor();
        let _ = telemetry.push(sample);

        // Drain buffered samples. On real hardware, a Primary would instead
        // send these out as `Data` messages via a `tpt_e_swarm_sync::mesh::
        // MeshNode` (which wraps its own chronos ring buffers for outbound/
        // inbound message queuing) rather than just discarding them here.
        while telemetry.pop().is_some() {}

        // -------------------------------------------------------------------
        // 3. Sleep — park until the next wake cycle.
        // -------------------------------------------------------------------
        // Real precondition-checked token issuance from other subsystems
        // doesn't exist yet (see `tpt-e-slumber`'s "Known limitations"), so
        // `Token::mock()` stands in here. `enter_deep_sleep` returns `!`, so
        // a real node's loop ends at the call below instead of continuing;
        // it's commented out so this scaffold stays runnable end to end.
        let dma_token = DmaParkedToken::mock();
        let rtc_token = RtcIsolatedToken::mock();
        let buffers_token = BuffersFlushedToken::mock();
        let _ = (&sleep_controller, dma_token, rtc_token, buffers_token);
        // sleep_controller.enter_deep_sleep(dma_token, rtc_token, buffers_token);

        break;
    }

    loop {}
}

/// Placeholder sensor read. Replace with a real ADC/I2C/SPI sensor driver.
fn read_sensor() -> u32 {
    0
}
