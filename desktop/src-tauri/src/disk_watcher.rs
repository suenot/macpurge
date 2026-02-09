//! DiskArbitration-based disk mount/unmount watcher
//!
//! Uses macOS DiskArbitration.framework to get instant notifications
//! when disks are mounted or unmounted, emitting Tauri events.

use core_foundation::base::*;
use core_foundation::runloop::*;
use core_foundation::string::CFStringRef;
use std::ffi::c_void;
use tauri::{AppHandle, Emitter};

// --- DiskArbitration FFI declarations ---

#[repr(C)]
struct __DASession(c_void);
type DASessionRef = *const __DASession;

#[repr(C)]
struct __DADisk(c_void);
type DADiskRef = *const __DADisk;

type DADiskCallback = extern "C" fn(disk: DADiskRef, context: *mut c_void);

// Opaque type for matching dictionary (we pass NULL to match all disks)
type CFDictionaryRef = *const c_void;

#[link(name = "DiskArbitration", kind = "framework")]
extern "C" {
    fn DASessionCreate(allocator: CFAllocatorRef) -> DASessionRef;
    fn DASessionScheduleWithRunLoop(
        session: DASessionRef,
        run_loop: CFRunLoopRef,
        mode: CFStringRef,
    );
    fn DARegisterDiskAppearedCallback(
        session: DASessionRef,
        match_dict: CFDictionaryRef,
        callback: DADiskCallback,
        context: *mut c_void,
    );
    fn DARegisterDiskDisappearedCallback(
        session: DASessionRef,
        match_dict: CFDictionaryRef,
        callback: DADiskCallback,
        context: *mut c_void,
    );
}

// --- Event payload ---

#[derive(Clone, serde::Serialize)]
pub struct DisksChangedPayload {
    pub reason: String,
}

// --- Watcher ---

pub struct DiskWatcher {
    app_handle: AppHandle,
}

impl DiskWatcher {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Start the DiskArbitration run loop in a dedicated thread.
    /// Fires "disks-changed" events on mount/unmount.
    pub fn start(self) {
        std::thread::spawn(move || {
            unsafe {
                let session = DASessionCreate(kCFAllocatorDefault);
                if session.is_null() {
                    eprintln!("DiskWatcher: failed to create DASession");
                    return;
                }

                // Leak AppHandle into a raw pointer for callbacks.
                // This is intentional — the watcher thread lives for the entire app lifetime.
                let context = Box::into_raw(Box::new(self.app_handle)) as *mut c_void;

                DARegisterDiskAppearedCallback(
                    session,
                    std::ptr::null(),
                    disk_appeared_cb,
                    context,
                );

                DARegisterDiskDisappearedCallback(
                    session,
                    std::ptr::null(),
                    disk_disappeared_cb,
                    context,
                );

                let run_loop = CFRunLoopGetCurrent();
                DASessionScheduleWithRunLoop(
                    session,
                    run_loop,
                    kCFRunLoopDefaultMode,
                );

                // Blocks forever — this is the event loop for DiskArbitration
                CFRunLoopRun();
            }
        });
    }
}

extern "C" fn disk_appeared_cb(_disk: DADiskRef, context: *mut c_void) {
    emit_disk_event(context, "mounted");
}

extern "C" fn disk_disappeared_cb(_disk: DADiskRef, context: *mut c_void) {
    emit_disk_event(context, "unmounted");
}

fn emit_disk_event(context: *mut c_void, reason: &str) {
    let app = unsafe { &*(context as *const AppHandle) };
    let _ = app.emit(
        "disks-changed",
        DisksChangedPayload {
            reason: reason.to_string(),
        },
    );
}
