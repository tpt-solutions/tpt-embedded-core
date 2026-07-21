#![allow(missing_docs)]

use proptest::prelude::*;
use tpt_e_chronos::ring_buf::RingBuf;

proptest! {
    #[test]
    fn push_pop_leaves_empty(items: Vec<u32>) {
        let buf = RingBuf::<u32, 16>::new(0);
        let mut pushed = 0;
        for &item in &items {
            if buf.push(item).is_ok() {
                pushed += 1;
            }
        }
        for _ in 0..pushed {
            let _ = buf.pop();
        }
        prop_assert!(buf.is_empty());
    }

    #[test]
    fn no_data_corruption(ops: Vec<bool>) {
        let buf = RingBuf::<u32, 8>::new(0);
        let mut expected: Vec<u32> = Vec::new();
        let mut counter = 0u32;

        for op in ops {
            if op {
                if buf.push(counter).is_ok() {
                    expected.push(counter);
                    counter += 1;
                }
            } else if let Some(val) = buf.pop() {
                let exp = expected.remove(0);
                prop_assert_eq!(val, exp);
            }
        }
    }

    #[test]
    fn never_exceeds_capacity(items: Vec<u32>) {
        let buf = RingBuf::<u32, 8>::new(0);
        let mut ok_count = 0;
        for &item in &items {
            if buf.push(item).is_ok() {
                ok_count += 1;
            }
        }
        prop_assert!(ok_count <= 8);
    }

    #[test]
    fn push_after_pop_wraps_around(count in 0u32..1000u32) {
        let buf = RingBuf::<u32, 4>::new(0);
        for i in 0..count {
            if buf.push(i).is_ok() {
                let _ = buf.pop();
            }
        }
        prop_assert!(buf.is_empty() || buf.len() <= 4);
    }
}
