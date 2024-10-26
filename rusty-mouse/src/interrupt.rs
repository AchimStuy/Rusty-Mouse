use wdk::{
    paged_code,println
};

use wdk_sys::{
    call_unsafe_wdf_function_binding,
    ntddk::KeGetCurrentIrql,
    APC_LEVEL,
    NTSTATUS,
    ULONG,
    WDFDEVICE,
    WDFINTERRUPT,
    WDF_INTERRUPT_CONFIG,
    WDF_NO_HANDLE,
    WDF_NO_OBJECT_ATTRIBUTES,
    WDF_OBJECT_ATTRIBUTES,
};

pub unsafe fn echo_interrupt_initialize(device: WDFDEVICE) -> NTSTATUS {
    let mut interrupt = WDF_NO_HANDLE as WDFINTERRUPT;

    println!("--> EchoInterruptInitialize");

    paged_code!();

    let mut interrupt_config = WDF_INTERRUPT_CONFIG {
        Size: core::mem::size_of::<WDF_INTERRUPT_CONFIG>() as ULONG,
        EvtInterruptIsr: Some(echo_evt_interrupt_isr),
        ..WDF_INTERRUPT_CONFIG::default()
    };

    let mut attributes = WDF_NO_OBJECT_ATTRIBUTES;

    // Create interrupt.
    let status = unsafe {
        call_unsafe_wdf_function_binding!(
            WdfInterruptCreate,
            device,
            &mut interrupt_config,
            attributes,
            &mut interrupt
        )
    };

    println!("<-- EchoInterruptInitialize resulted with {status:#010X}");
    status
}

extern "C" fn echo_evt_interrupt_isr(interrupt: WDFINTERRUPT, message_id: ULONG) -> u8 {
    println!("echo_evt_interrupt_isr called! interrupt {:?}, message id {:?}", interrupt, message_id);
    true as u8
}