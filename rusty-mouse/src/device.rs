// Copyright (c) Microsoft Corporation.
// License: MIT OR Apache-2.0

use wdk::{nt_success, paged_code, println};
use wdk_sys::{
    call_unsafe_wdf_function_binding,
    APC_LEVEL,
    NTSTATUS,
    STATUS_SUCCESS,
    ULONG,
    WDFDEVICE,
    WDFDEVICE_INIT,
    WDFOBJECT,
    WDFQUEUE,
    WDF_NO_HANDLE,
    WDF_OBJECT_ATTRIBUTES,
    WDF_PNPPOWER_EVENT_CALLBACKS,
    _WDF_EXECUTION_LEVEL,
    _WDF_SYNCHRONIZATION_SCOPE,
};

use crate::{
    queue::echo_queue_initialize,
    queue_get_context,
    wdf_object_context::wdf_get_context_type_info,
    wdf_object_get_device_context,
    DeviceContext,
    KeGetCurrentIrql,
    GUID_DEVINTERFACE_ECHO,
    WDF_DEVICE_CONTEXT_TYPE_INFO,
    WDF_REQUEST_CONTEXT_TYPE_INFO,
};

/// Worker routine called to create a device and its software resources.
///
/// # Arguments:
///
/// * `device_init` - Pointer to an opaque init structure. Memory for this
///   structure will be freed by the framework when the `WdfDeviceCreate`
///   succeeds. So don't access the structure after that point.
///
/// # Return value:
///
/// * `NTSTATUS`
#[link_section = "PAGE"]
pub fn echo_device_create(mut device_init: &mut WDFDEVICE_INIT) -> NTSTATUS {
    paged_code!();

    // Register pnp/power callbacks so that we can start and stop the timer as the
    // device gets started and stopped.
    let mut pnp_power_callbacks = WDF_PNPPOWER_EVENT_CALLBACKS {
        Size: core::mem::size_of::<WDF_PNPPOWER_EVENT_CALLBACKS>() as ULONG,
        EvtDeviceSelfManagedIoInit: Some(echo_evt_device_self_managed_io_start),
        EvtDeviceSelfManagedIoSuspend: Some(echo_evt_device_self_managed_io_suspend),
        // Function used for both Init and Restart Callbacks
        EvtDeviceSelfManagedIoRestart: Some(echo_evt_device_self_managed_io_start),
        ..WDF_PNPPOWER_EVENT_CALLBACKS::default()
    };

    // Register the PnP and power callbacks. Power policy related callbacks will be
    // registered later in SotwareInit.
    unsafe {
        call_unsafe_wdf_function_binding!(
            WdfDeviceInitSetPnpPowerEventCallbacks,
            device_init,
            &mut pnp_power_callbacks
        );
    };

    let mut attributes = WDF_OBJECT_ATTRIBUTES {
        Size: core::mem::size_of::<WDF_OBJECT_ATTRIBUTES>() as ULONG,
        ExecutionLevel: _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent,
        SynchronizationScope: _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent,
        ContextTypeInfo: wdf_get_context_type_info!(RequestContext),
        ..WDF_OBJECT_ATTRIBUTES::default()
    };

    unsafe {
        call_unsafe_wdf_function_binding!(
            WdfDeviceInitSetRequestAttributes,
            device_init,
            &mut attributes
        );
    };

    let mut attributes = WDF_OBJECT_ATTRIBUTES {
        Size: core::mem::size_of::<WDF_OBJECT_ATTRIBUTES>() as ULONG,
        ExecutionLevel: _WDF_EXECUTION_LEVEL::WdfExecutionLevelInheritFromParent,
        SynchronizationScope: _WDF_SYNCHRONIZATION_SCOPE::WdfSynchronizationScopeInheritFromParent,
        ContextTypeInfo: wdf_get_context_type_info!(DeviceContext),
        ..WDF_OBJECT_ATTRIBUTES::default()
    };

    let mut device = WDF_NO_HANDLE as WDFDEVICE;
    let mut status = unsafe {
        call_unsafe_wdf_function_binding!(
            WdfDeviceCreate,
            (core::ptr::addr_of_mut!(device_init)) as *mut *mut WDFDEVICE_INIT,
            &mut attributes,
            &mut device,
        )
    };

    if nt_success(status) {
        // Get the device context and initialize it. WdfObjectGet_DEVICE_CONTEXT is an
        // inline function generated by WDF_DECLARE_CONTEXT_TYPE macro in the
        // device.h header file. This function will do the type checking and return
        // the device context. If you pass a wrong object  handle
        // it will return NULL and assert if run under framework verifier mode.
        let device_context: *mut DeviceContext =
            unsafe { wdf_object_get_device_context(device as WDFOBJECT) };
        unsafe { (*device_context).private_device_data = 0 };

        // Create a device interface so that application can find and talk
        // to us.
        status = unsafe {
            call_unsafe_wdf_function_binding!(
                WdfDeviceCreateDeviceInterface,
                device,
                &GUID_DEVINTERFACE_ECHO,
                core::ptr::null_mut(),
            )
        };

        if nt_success(status) {
            // Initialize the I/O Package and any Queues
            status = unsafe { echo_queue_initialize(device) };
        }
    }
    status
}

/// This event is called by the Framework when the device is started
/// or restarted after a suspend operation.
///
/// This function is not marked pageable because this function is in the
/// device power up path. When a function is marked pagable and the code
/// section is paged out, it will generate a page fault which could impact
/// the fast resume behavior because the client driver will have to wait
/// until the system drivers can service this page fault.
///
/// # Arguments:
///
/// * `device` - Handle to a framework device object.
///
/// # Return value:
///
/// * `NTSTATUS` - Failures will result in the device stack being torn down.
extern "C" fn echo_evt_device_self_managed_io_start(device: WDFDEVICE) -> NTSTATUS {
    // Restart the queue and the periodic timer. We stopped them before going
    // into low power state.
    let queue: WDFQUEUE;

    println!("--> EchoEvtDeviceSelfManagedIoInit");

    unsafe {
        queue = call_unsafe_wdf_function_binding!(WdfDeviceGetDefaultQueue, device);
    };

    let queue_context = unsafe { queue_get_context(queue as WDFOBJECT) };

    // Restart the queue and the periodic timer. We stopped them before going
    // into low power state.
    unsafe { call_unsafe_wdf_function_binding!(WdfIoQueueStart, queue) };

    let due_time: i64 = -(100) * (10000);

    let _ = unsafe { (*queue_context).timer.start(due_time) };

    println!("<-- EchoEvtDeviceSelfManagedIoInit");

    STATUS_SUCCESS
}

/// This event is called by the Framework when the device is stopped
/// for resource rebalance or suspended when the system is entering
/// Sx state.
///
/// # Arguments:
///
/// * `device` - Handle to a framework device object.
///
/// # Return value:
///
/// * `NTSTATUS` - The driver is not allowed to fail this function.  If it does,
///   the device stack will be torn down.
#[link_section = "PAGE"]
unsafe extern "C" fn echo_evt_device_self_managed_io_suspend(device: WDFDEVICE) -> NTSTATUS {
    paged_code!();

    println!("--> EchoEvtDeviceSelfManagedIoSuspend");

    // Before we stop the timer we should make sure there are no outstanding
    // i/o. We need to do that because framework cannot suspend the device
    // if there are requests owned by the driver. There are two ways to solve
    // this issue: 1) We can wait for the outstanding I/O to be complete by the
    // periodic timer 2) Register EvtIoStop callback on the queue and acknowledge
    // the request to inform the framework that it's okay to suspend the device
    // with outstanding I/O. In this sample we will use the 1st approach
    // because it's pretty easy to do. We will restart the queue when the
    // device is restarted.
    let queue = unsafe { call_unsafe_wdf_function_binding!(WdfDeviceGetDefaultQueue, device) };
    let queue_context = unsafe { queue_get_context(queue as WDFOBJECT) };

    unsafe {
        call_unsafe_wdf_function_binding!(WdfIoQueueStopSynchronously, queue);
        // Stop the watchdog timer and wait for DPC to run to completion if it's already
        // fired.
        let _ = (*queue_context).timer.stop(true);
    };

    println!("<-- EchoEvtDeviceSelfManagedIoSuspend");

    STATUS_SUCCESS
}