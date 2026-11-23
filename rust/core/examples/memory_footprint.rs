// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Requested live Rust heap bytes, independently of OS working-set noise.
//! Instrumentation is limited to this example; do not use it for speed timing.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

static LIVE: AtomicUsize = AtomicUsize::new(0);
struct Counted;
#[global_allocator]
static ALLOCATOR: Counted = Counted;

unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, size) };
        if !result.is_null() {
            if size >= layout.size() {
                LIVE.fetch_add(size - layout.size(), Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - size, Relaxed);
            }
        }
        result
    }
}

fn main() {
    let initial = LIVE.load(Relaxed);
    black_box(meltype_core::dictionary::Dictionary::bundled());
    let bundled = LIVE.load(Relaxed);
    let profile = black_box(meltype_core::dictionary::Dictionary::bundled().clone());
    let cloned = LIVE.load(Relaxed);
    println!(
        "{}",
        serde_json::json!({
            "bundled_requested_live_bytes": bundled - initial,
            "cloned_dictionary_requested_live_bytes": cloned - bundled,
            "scope": "Rust allocations requested by dictionary initialization and clone; excludes allocator overhead, OS RSS, static bundled bytes and non-Rust allocators. Not a speed measurement."
        })
    );
    black_box(profile);
}
