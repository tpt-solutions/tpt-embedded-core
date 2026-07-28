#![cfg(feature = "mock")]
#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_typestate_hal::backend::IsrOps;
use tpt_e_typestate_hal::isr::IsrGuard;
use tpt_e_typestate_hal::mock::MockIsrGuard;

/// Registers a handler through the `IsrOps<F>` trait, generic over the
/// backend `B` — exercises the trait itself (not `MockIsrGuard`'s inherent
/// `new`), proving `MockIsrGuard<F>` genuinely implements `IsrOps<F>`.
#[allow(unsafe_code)] // test-only: exercising an `unsafe trait fn`, not raw register access
fn register_via_trait<F: Fn(), B: IsrOps<F>>(handler: F) -> B {
    // SAFETY: test-only handler does no blocking/allocation.
    unsafe { B::register(handler) }
}

#[test]
fn mock_isr_guard_implements_isr_ops_trait() {
    use core::sync::atomic::{AtomicBool, Ordering};
    static CALLED: AtomicBool = AtomicBool::new(false);

    let guard: MockIsrGuard<_> = register_via_trait(|| {
        CALLED.store(true, Ordering::Relaxed);
    });
    guard.call();
    assert!(CALLED.load(Ordering::Relaxed));
}

proptest! {
    #[test]
    fn mock_isr_guard_creation_and_call(val in 0u32..1000) {
        let guard = IsrGuard::mock(move || {
            let _ = val;
        });
        let _ = format!("{:?}", guard);
        guard.call_mock();
    }

    #[test]
    fn mock_isr_guard_drop_does_not_panic(val in 0u32..1000) {
        let guard = IsrGuard::mock(move || {
            let _ = val;
        });
        drop(guard);
    }

    #[test]
    fn mock_isr_guard_handler_called_exactly_once(val in 0u32..1000) {
        use core::sync::atomic::{AtomicU32, Ordering};
        static CALL_COUNT: AtomicU32 = AtomicU32::new(0);
        CALL_COUNT.store(0, Ordering::Relaxed);

        let guard = IsrGuard::mock(move || {
            let _ = val;
            let _ = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        });
        guard.call_mock();
        guard.call_mock();
        assert_eq!(CALL_COUNT.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn mock_isr_guard_multiple_independent_guards(channel_id in 0u8..8) {
        // Create multiple guards with different captured values
        let guard1 = IsrGuard::mock(move || { let _ = channel_id; });
        let guard2 = IsrGuard::mock(move || { let _ = channel_id; });
        guard1.call_mock();
        guard2.call_mock();
        drop(guard1);
        drop(guard2);
    }
}
