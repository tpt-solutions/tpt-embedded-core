# Cross-Crate Wiring

The five crates are designed to compose. Here's how they connect:

## Dependency Graph

```text
tpt-e-swarm-sync  ──→  tpt-e-chronos  ──→  tpt-e-typestate-hal (optional)
                                         └→  esp-hal (optional)
tpt-e-cipher         (standalone)
tpt-e-slumber        (standalone)
```

## Ring Buffer + DMA Transfer

The most common integration is feeding a `tpt-e-chronos` ring buffer
into a `tpt-e-typestate-hal` DMA transfer:

```rust,ignore
use tpt_e_chronos::ring_buf::RingBuf;
use tpt_e_chronos::dma_handoff::transfer_with_dma;
use tpt_e_typestate_hal::dma::DmaChannel;

let mut ring = RingBuf::<u32, 4>::new(0);
let _ = ring.push(42);

let channel = DmaChannel::<_, MockDmaChannel>::mock(0);
let mut ring = unsafe { transfer_with_dma(&mut ring, channel) };
assert_eq!(ring.pop(), Some(42));
```

## Mesh Node + Ring Buffer

`tpt-e-swarm-sync` uses `tpt-e-chronos` internally for message queuing:

```rust,ignore
use tpt_e_swarm_sync::mesh::MeshNode;

let mut node = MeshNode::new(1);
// Ring buffers are used internally for inbound/outbound message queues
```

## Sleep Tokens + DMA State

`tpt-e-slumber` tokens are issued by `tpt-e-typestate-hal` when DMA
channels reach the `Complete` or `Idle` state:

```rust,ignore
use tpt_e_slumber::sleep::SleepController;

// After all DMA channels are parked:
let dma_token = DmaParkedToken::mock(); // issued by typestate-hal
let rtc_token = RtcIsolatedToken::mock();
let buf_token = BuffersFlushedToken::mock();

let controller = SleepController::new();
// controller.enter_deep_sleep(dma_token, rtc_token, buf_token);
```
