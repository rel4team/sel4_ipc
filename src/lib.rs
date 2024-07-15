//! This crate implements the IPC mechanism of seL4, including the endpoint, notification, and transfer.
//!
//! See more details in ../doc.md
#![feature(core_intrinsics)]
#![no_std]
#![allow(internal_features)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::tests::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![no_main]
#![feature(asm_const)]

mod endpoint;
mod notification;
mod transfer;

pub use endpoint::*;
pub use notification::*;
pub use transfer::*;

#[cfg(test)]
mod tests {
    use core::arch::global_asm;
    use riscv::register::{stvec, utvec::TrapMode};
    use sel4_common::{
        arch::{shutdown, ArchReg, ArchTCB},
        fault::{lookup_fault_t, seL4_Fault_t},
        println,
    };
    use sel4_task::{tcb_t, thread_state_t, ThreadState};
    global_asm!(include_str!("entry.asm"));

    use super::*;

    fn new_mock_tcb_with_state(state: ThreadState) -> tcb_t {
        tcb_t {
            tcbEPPrev: 0,
            tcbEPNext: 0,
            tcbSchedPrev: 0,
            tcbSchedNext: 0,
            tcbIPCBuffer: 0,
            tcbFaultHandler: 0,
            tcbTimeSlice: 0,
            tcbPriority: 0,
            tcbMCP: 0,
            domain: 0,
            tcbLookupFailure: lookup_fault_t::new_root_invalid(),
            tcbFault: seL4_Fault_t::new_null_fault(),
            tcbBoundNotification: 0,
            tcbState: thread_state_t::state_new(0, 0, 0, 0, 0, 0, state as usize),
            tcbArch: ArchTCB::default(),
        }
    }

    pub fn test_runner(tests: &[&dyn Fn()]) {
        println!("Running {} tests\n", tests.len());
        for test in tests {
            test();
        }
        println!("All Test Cases(count: {}) passed!", tests.len());
        shutdown();
    }

    #[test_case]
    pub fn endpoint_create_happy_case_test() {
        println!(">>>>>>>>>>>> Entering endpoint_create_happy_case_test...");

        let head = 0x8000;
        let tail = 0x9000;
        let ep = endpoint_t::new(head, tail, EPState::Idle as usize);
        assert_eq!(ep.get_state(), EPState::Idle);
        assert_eq!(ep.get_queue_head(), head);
        assert_eq!(ep.get_queue_tail(), tail);

        let ep = endpoint_t::new(head, tail, EPState::Send as usize);
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), head);
        assert_eq!(ep.get_queue_tail(), tail);

        let ep = endpoint_t::new(head, tail, EPState::Recv as usize);
        assert_eq!(ep.get_state(), EPState::Recv);
        assert_eq!(ep.get_queue_head(), head);
        assert_eq!(ep.get_queue_tail(), tail);

        println!("Test endpoint_create_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_set_and_get_state_test() {
        println!(">>>>>>>>>>>> Entering endpoint_set_and_get_state_test...");

        let head = 0x8000;
        let tail = 0x9000;
        let mut ep = endpoint_t::new(head, tail, EPState::Idle as usize);
        assert_eq!(ep.get_state(), EPState::Idle);
        ep.set_state(EPState::Send as _);
        assert_eq!(ep.get_state(), EPState::Send);
        ep.set_state(EPState::Recv as _);
        assert_eq!(ep.get_state(), EPState::Recv);

        println!("Test endpoint_set_and_get_state_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_set_and_get_queue_head_test() {
        println!(">>>>>>>>>>>> Entering endpoint_set_and_get_queue_head_test...");

        let head = 0x8000;
        let tail = 0x9000;
        let mut ep = endpoint_t::new(head, tail, EPState::Idle as usize);
        assert_eq!(ep.get_queue_head(), head);
        ep.set_queue_head(0x9000);
        assert_eq!(ep.get_queue_head(), 0x9000);
        ep.set_queue_head(0x8000);
        assert_eq!(ep.get_queue_head(), 0x8000);

        println!("Test endpoint_set_and_get_queue_head_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_set_and_get_queue_tail_test() {
        println!(">>>>>>>>>>>> Entering endpoint_set_and_get_queue_tail_test...");

        let head = 0x8000;
        let tail = 0x9000;
        let mut ep = endpoint_t::new(head, tail, EPState::Idle as usize);
        assert_eq!(ep.get_queue_tail(), tail);
        ep.set_queue_tail(0x8000);
        assert_eq!(ep.get_queue_tail(), 0x8000);
        ep.set_queue_tail(0x9000);
        assert_eq!(ep.get_queue_tail(), 0x9000);

        println!("Test endpoint_set_and_get_queue_tail_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_send_and_cancel_ipc_should_be_empty_test() {
        println!(">>>>>>>>>>>> Entering endpoint_send_and_cancel_ipc_should_be_empty_test...");

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.send_ipc(mock_tcb, true, false, false, 0, false); // send
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), mock_tcb.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb.get_ptr());

        ep.cancel_ipc(mock_tcb);
        assert_eq!(ep.get_state(), EPState::Idle);
        assert_eq!(ep.get_queue_head(), 0);
        assert_eq!(ep.get_queue_tail(), 0);
        assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateInactive);

        println!("Test endpoint_send_and_cancel_ipc_should_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_cancel_all_ipc_happy_case_test() {
        println!(">>>>>>>>>>>> Entering endpoint_cancel_all_ipc_happy_case_test...");

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mut mock_tcbs = [
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
        ];

        for mock_tcb in mock_tcbs.iter_mut() {
            ep.send_ipc(mock_tcb, true, false, false, 0, false); // send
        }
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), mock_tcbs[0].get_ptr());
        assert_eq!(
            ep.get_queue_tail(),
            mock_tcbs[mock_tcbs.len() - 1].get_ptr()
        );

        ep.cancel_all_ipc();
        assert_eq!(ep.get_state(), EPState::Idle);
        assert_eq!(ep.get_queue_head(), 0);
        assert_eq!(ep.get_queue_tail(), 0);

        for mock_tcb in mock_tcbs.into_iter() {
            assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateRestart);
        }

        println!("Test endpoint_cancel_all_ipc_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_send_and_cancel_ipc_queue_should_not_be_empty_test() {
        println!(
            ">>>>>>>>>>>> Entering endpoint_send_and_cancel_ipc_queue_should_not_be_empty_test..."
        );

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.send_ipc(mock_tcb1, true, false, false, 0, false); // send
        ep.send_ipc(mock_tcb2, true, false, false, 0, false); // send
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), mock_tcb1.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb2.get_ptr());

        ep.cancel_ipc(mock_tcb1);
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), mock_tcb2.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb2.get_ptr());
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateInactive);
        assert_eq!(mock_tcb2.get_state(), ThreadState::ThreadStateBlockedOnSend);

        println!("Test endpoint_send_and_cancel_ipc_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_receive_and_cancel_ipc_should_be_empty_test() {
        println!(">>>>>>>>>>>> Entering endpoint_receive_and_cancel_ipc_should_be_empty_test...");

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.receive_ipc(mock_tcb, true, false); // receive
        assert_eq!(ep.get_state(), EPState::Recv);
        assert_eq!(ep.get_queue_head(), mock_tcb.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb.get_ptr());

        ep.cancel_ipc(mock_tcb);
        assert_eq!(ep.get_state(), EPState::Idle);
        assert_eq!(ep.get_queue_head(), 0);
        assert_eq!(ep.get_queue_tail(), 0);
        assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateInactive);

        println!("Test endpoint_receive_and_cancel_ipc_should_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_receive_and_cancel_ipc_queue_should_not_be_empty_test() {
        println!(">>>>>>>>>>>> Entering endpoint_receive_and_cancel_ipc_queue_should_not_be_empty_test...");

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.receive_ipc(mock_tcb1, true, false); // receive
        ep.receive_ipc(mock_tcb2, true, false); // receive
        assert_eq!(ep.get_state(), EPState::Recv);
        assert_eq!(ep.get_queue_head(), mock_tcb1.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb2.get_ptr());

        ep.cancel_ipc(mock_tcb1);
        assert_eq!(ep.get_state(), EPState::Recv);
        assert_eq!(ep.get_queue_head(), mock_tcb2.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb2.get_ptr());
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateInactive);
        assert_eq!(
            mock_tcb2.get_state(),
            ThreadState::ThreadStateBlockedOnReceive
        );

        println!("Test endpoint_receive_and_cancel_ipc_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_send_and_receive_ipc_queue_should_be_empty_test() {
        println!(
            ">>>>>>>>>>>> Entering endpoint_send_and_receive_ipc_queue_should_not_be_empty_test..."
        );

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.send_ipc(mock_tcb1, true, false, false, 0, false); // send
        assert_eq!(ep.get_state(), EPState::Send);
        ep.receive_ipc(mock_tcb2, true, false); // receive
        assert_eq!(ep.get_state(), EPState::Idle);
        assert_eq!(ep.get_queue_head(), 0);
        assert_eq!(ep.get_queue_tail(), 0);
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateRunning);
        assert_eq!(mock_tcb2.get_state(), ThreadState::ThreadStateRunning);

        println!("Test endpoint_send_and_receive_ipc_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn endpoint_send_and_receive_ipc_queue_should_not_be_empty_test() {
        println!(
            ">>>>>>>>>>>> Entering endpoint_send_and_receive_ipc_queue_should_not_be_empty_test..."
        );

        let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb3 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ep.send_ipc(mock_tcb1, true, false, false, 0, false); // send
        ep.send_ipc(mock_tcb2, true, false, false, 0, false); // send
        assert_eq!(ep.get_state(), EPState::Send);
        ep.receive_ipc(mock_tcb3, true, false); // receive
        assert_eq!(ep.get_state(), EPState::Send);
        assert_eq!(ep.get_queue_head(), mock_tcb2.get_ptr());
        assert_eq!(ep.get_queue_tail(), mock_tcb2.get_ptr());
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateRunning);
        assert_eq!(mock_tcb2.get_state(), ThreadState::ThreadStateBlockedOnSend);
        assert_eq!(mock_tcb3.get_state(), ThreadState::ThreadStateRunning);

        println!("Test endpoint_send_and_receive_ipc_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_create_test() {
        println!(">>>>>>>>>>>> Entering notification_create_test...");

        let head = 0x8000;
        let tail = 0x9000;
        let badge = 0x1000;
        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let ntfn = notification_t::new(
            mock_tcb as *const tcb_t as usize,
            badge,
            head,
            tail,
            NtfnState::Idle as usize,
        );
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        assert_eq!(ntfn.get_queue_head(), head);
        assert_eq!(ntfn.get_queue_tail(), tail);
        assert_eq!(ntfn.get_bound_tcb(), mock_tcb.get_ptr());
        assert_eq!(ntfn.get_msg_identifier(), badge);

        println!("Test notification_create_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_set_and_get_state_happy_case_test() {
        println!(">>>>>>>>>>>> Entering notification_set_and_get_state_happy_case_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        ntfn.set_state(NtfnState::Waiting as _);
        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        ntfn.set_state(NtfnState::Active as _);
        assert_eq!(ntfn.get_state(), NtfnState::Active);

        println!("Test notification_set_and_get_state_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_set_and_get_queue_happy_case_test() {
        println!(">>>>>>>>>>>> Entering notification_set_and_get_queue_happy_case_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        assert_eq!(ntfn.get_queue_head(), 0);
        assert_eq!(ntfn.get_queue_tail(), 0);
        let head = 0x8000;
        let tail = 0x9000;
        ntfn.set_queue_head(head);
        ntfn.set_queue_tail(tail);
        assert_eq!(ntfn.get_queue_head(), head);
        assert_eq!(ntfn.get_queue_tail(), tail);

        println!("Test notification_set_and_get_queue_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_active_happy_case_test() {
        println!(">>>>>>>>>>>> Entering notification_active_happy_case_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        let badge = 0x1000;
        ntfn.active(badge);
        assert_eq!(ntfn.get_state(), NtfnState::Active);
        assert_eq!(ntfn.get_msg_identifier(), badge);

        println!("Test notification_active_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_bind_and_unbind_tcb_happy_case_test() {
        println!(">>>>>>>>>>>> Entering notification_bind_and_unbind_tcb_happy_case_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ntfn.bind_tcb(mock_tcb);
        assert_eq!(ntfn.get_bound_tcb(), mock_tcb.get_ptr());
        ntfn.unbind_tcb();
        assert_eq!(ntfn.get_bound_tcb(), 0);

        ntfn.bind_tcb(mock_tcb);
        ntfn.safe_unbind_tcb();
        assert_eq!(ntfn.get_bound_tcb(), 0);

        println!("Test notification_bind_and_unbind_tcb_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[no_mangle]
    pub fn post_cap_deletion() {}
    #[no_mangle]
    pub fn finaliseCap() {}

    #[test_case]
    pub fn endpoint_send_and_cancel_signal_queue_should_not_be_empty_test() {
        println!(">>>>>>>>>>>> Entering endpoint_send_and_cancel_signal_queue_should_not_be_empty_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ntfn.receive_signal(mock_tcb, true); // receive
        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        assert_eq!(ntfn.get_queue_head(), mock_tcb.get_ptr());
        assert_eq!(ntfn.get_queue_tail(), mock_tcb.get_ptr());

        ntfn.cancel_signal(mock_tcb);
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        assert_eq!(ntfn.get_queue_head(), 0);
        assert_eq!(ntfn.get_queue_tail(), 0);
        assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateInactive);

        println!("Test endpoint_send_and_cancel_signal_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_receive_and_cancel_all_signal_should_be_empty_test() {
        println!(">>>>>>>>>>>> Entering notification_receive_and_cancel_all_signal_should_be_empty_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        let mut mock_tcbs = [
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
            &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning),
        ];

        for mock_tcb in mock_tcbs.iter_mut() {
            ntfn.receive_signal(mock_tcb, true); // receive
        }

        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        assert_eq!(ntfn.get_queue_head(), mock_tcbs[0].get_ptr());
        assert_eq!(
            ntfn.get_queue_tail(),
            mock_tcbs[mock_tcbs.len() - 1].get_ptr()
        );

        ntfn.cacncel_all_signal();
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        assert_eq!(ntfn.get_queue_head(), 0);
        assert_eq!(ntfn.get_queue_tail(), 0);

        for mock_tcb in mock_tcbs.into_iter() {
            assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateRestart);
        }

        println!("Test notification_receive_and_cancel_all_signal_should_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_receive_and_send_signal_queue_should_be_empty_test() {
        println!(">>>>>>>>>>>> Entering notification_receive_and_send_signal_queue_should_be_empty_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ntfn.receive_signal(mock_tcb1, true); // receive
        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        assert_eq!(ntfn.get_queue_head(), mock_tcb1.get_ptr());
        assert_eq!(ntfn.get_queue_tail(), mock_tcb1.get_ptr());

        ntfn.send_signal(mock_tcb2.get_ptr());
        assert_eq!(ntfn.get_state(), NtfnState::Idle);
        assert_eq!(ntfn.get_queue_head(), 0);
        assert_eq!(ntfn.get_queue_tail(), 0);
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateRunning);
        assert_eq!(mock_tcb2.get_state(), ThreadState::ThreadStateRunning);

        println!("Test notification_receive_and_send_signal_queue_should_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn notification_receive_and_send_signal_queue_should_not_be_empty_test() {
        println!(">>>>>>>>>>>> Entering notification_receive_and_send_signal_queue_should_not_be_empty_test...");

        let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
        let mock_tcb1 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb2 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_tcb3 = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        ntfn.receive_signal(mock_tcb1, true); // receive
        ntfn.receive_signal(mock_tcb2, true); // receive
        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        assert_eq!(ntfn.get_queue_head(), mock_tcb1.get_ptr());
        assert_eq!(ntfn.get_queue_tail(), mock_tcb2.get_ptr());

        ntfn.send_signal(mock_tcb3.get_ptr());
        assert_eq!(ntfn.get_state(), NtfnState::Waiting);
        assert_eq!(ntfn.get_queue_head(), mock_tcb2.get_ptr());
        assert_eq!(ntfn.get_queue_tail(), mock_tcb2.get_ptr());
        assert_eq!(mock_tcb1.get_state(), ThreadState::ThreadStateRunning);
        assert_eq!(
            mock_tcb2.get_state(),
            ThreadState::ThreadStateBlockedOnNotification
        );
        assert_eq!(mock_tcb3.get_state(), ThreadState::ThreadStateRunning);

        println!("Test notification_receive_and_send_signal_queue_should_not_be_empty_test passed!<<<<<<<<<<<<\n");
    }

    // without call
    #[test_case]
    pub fn transfer_cancel_ipc_happy_case_test() {
        println!(">>>>>>>>>>>> Entering transfer_cancel_ipc_happy_case_test...");

        let mock_tcb = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        // endpoint
        {
            let mut ep = endpoint_t::new(0, 0, EPState::Idle as usize);
            ep.send_ipc(mock_tcb, true, false, false, 0, false); // send
            assert_eq!(ep.get_state(), EPState::Send);
            assert_eq!(ep.get_queue_head(), mock_tcb.get_ptr());
            assert_eq!(ep.get_queue_tail(), mock_tcb.get_ptr());
            assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateBlockedOnSend);

            mock_tcb.cancel_ipc();
            assert_eq!(ep.get_state(), EPState::Idle);
            assert_eq!(ep.get_queue_head(), 0);
            assert_eq!(ep.get_queue_tail(), 0);
            assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateInactive);
        }
        // notification
        {
            let mut ntfn = notification_t::new(0, 0, 0, 0, NtfnState::Idle as usize);
            ntfn.receive_signal(mock_tcb, true); // receive
            assert_eq!(ntfn.get_state(), NtfnState::Waiting);
            assert_eq!(ntfn.get_queue_head(), mock_tcb.get_ptr());
            assert_eq!(ntfn.get_queue_tail(), mock_tcb.get_ptr());
            assert_eq!(
                mock_tcb.get_state(),
                ThreadState::ThreadStateBlockedOnNotification
            );

            mock_tcb.cancel_ipc();
            assert_eq!(ntfn.get_state(), NtfnState::Idle);
            assert_eq!(ntfn.get_queue_head(), 0);
            assert_eq!(ntfn.get_queue_tail(), 0);
            assert_eq!(mock_tcb.get_state(), ThreadState::ThreadStateInactive);
        }

        println!("Test transfer_cancel_ipc_happy_case_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn transfer_set_fault_mrs_CapFault_test() {
        println!(">>>>>>>>>>>> Entering transfer_set_fault_mrs_CapFault_test...");

        let mut mock_tcb = new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_receiver = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);

        let mock_fault = seL4_Fault_t::new_cap_fault(0x1000, 0);
        let mock_fault_ip = 0x2000;
        mock_tcb.tcbFault = mock_fault;
        mock_tcb
            .tcbArch
            .set_register(ArchReg::FaultIP, mock_fault_ip);
        mock_tcb.set_fault_mrs(mock_receiver);

        let offset = 0;
        assert_eq!(
            mock_receiver.tcbArch.get_register(ArchReg::Msg(offset)),
            0x2000
        );

        println!("Test transfer_set_fault_mrs_CapFault_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn transfer_set_fault_mrs_UserException_test() {
        println!(">>>>>>>>>>>> Entering transfer_set_fault_mrs_UserException_test...");

        let mut mock_tcb = new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_receiver = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);

        let mock_fault = seL4_Fault_t::new_user_exeception(0x1000, 0);
        let mock_fault_ip = 0x2000;
        mock_tcb.tcbFault = mock_fault;
        mock_tcb
            .tcbArch
            .set_register(ArchReg::FaultIP, mock_fault_ip);
        mock_tcb.set_fault_mrs(mock_receiver);

        let offset = 0;
        assert_eq!(
            mock_receiver.tcbArch.get_register(ArchReg::Msg(offset)),
            0x2000
        );

        println!("Test transfer_set_fault_mrs_UserException_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn transfer_set_fault_mrs_VMFault_test() {
        println!(">>>>>>>>>>>> Entering transfer_set_fault_mrs_VMFault_test...");

        let mut mock_tcb = new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_receiver = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);

        let mock_fault = seL4_Fault_t::new_vm_fault(0x1000, 0, 0);
        let mock_fault_ip = 0x2000;
        mock_tcb.tcbFault = mock_fault;
        mock_tcb
            .tcbArch
            .set_register(ArchReg::FaultIP, mock_fault_ip);
        mock_tcb.set_fault_mrs(mock_receiver);

        let offset = 0;
        assert_eq!(
            mock_receiver.tcbArch.get_register(ArchReg::Msg(offset)),
            0x2000
        );

        println!("Test transfer_set_fault_mrs_VMFault_test passed!<<<<<<<<<<<<\n");
    }

    #[test_case]
    pub fn transfer_set_fault_mrs_UnknownSyscall_test() {
        println!(">>>>>>>>>>>> Entering transfer_set_fault_mrs_UnknownSyscall_test...");

        let mut mock_tcb = new_mock_tcb_with_state(ThreadState::ThreadStateRunning);
        let mock_receiver = &mut new_mock_tcb_with_state(ThreadState::ThreadStateRunning);

        let mock_fault = seL4_Fault_t::new_unknown_syscall_fault(0x1000);
        let mock_fault_ip = 0x2000;
        mock_tcb.tcbFault = mock_fault;
        mock_tcb
            .tcbArch
            .set_register(ArchReg::FaultIP, mock_fault_ip);
        mock_tcb.set_fault_mrs(mock_receiver);

        let offset = 0;
        assert_eq!(
            mock_receiver.tcbArch.get_register(ArchReg::Msg(offset)),
            0x2000
        );

        println!("Test transfer_set_fault_mrs_UnknownSyscall_test passed!<<<<<<<<<<<<\n");
    }

    // TODO: do_transfer relevant tests supplyment

    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        println!("{}", info);
        shutdown()
    }

    #[no_mangle]
    pub fn call_test_main() {
        extern "C" {
            fn trap_entry();
        }
        unsafe {
            stvec::write(trap_entry as usize, TrapMode::Direct);
        }
        crate::test_main();
    }
    #[no_mangle]
    pub fn c_handle_syscall() {
        unsafe {
            core::arch::asm!("sret");
        }
    }
}
