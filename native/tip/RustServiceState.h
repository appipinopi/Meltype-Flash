// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 lnkiai, appipinopi
#pragma once
#include "Globals.h"
#include <cstddef>
#include <type_traits>
#ifdef MELTYPE_RUST_CORE
namespace meltype {
// Must match Rust ServiceState. Allocation checks all offsets before access.
struct RustServiceState {
    LONG refs;
    ITfThreadMgr* manager;
    TfClientId client;
    void* thread_subscription;
    void* key_subscription;
    void* mode_subscription;
    void* edit_subscription;
    ITfContext* edit_context;
    ITfComposition* composition;
    ITfContext* composition_context;
    TfGuidAtom atom_input;
    TfGuidAtom atom_converted;
    TfGuidAtom atom_target;
    void* keys;
    void* resume;
    void* abandoned;
    ULONGLONG last_hello;
    unsigned long long focus_epoch;
    unsigned long long mode_epoch;
    unsigned int shutdown_phase;
    bool server_active;
    bool direct_detection;
    bool direct_word;
    bool open;
    bool code_app;
    bool code_english;
    bool disabled;
    bool own_process;
    bool last_converting;
    bool caret_moved;
    bool own_edit;
    bool secret_checked;
    bool secret_composing;
    bool private_field;
    bool keep_caret;
    bool deactivated;
    bool key_failed;
    bool consumed;
    const wchar_t* scope_source;
    const wchar_t* pass_reason;
    char* sid; // Rust-owned; access only through the service API.
    void* transport; // Rust-owned transport token.
    void* langbar; // Rust-owned COM reference.
    void* candidates; // Rust-owned thread-local window token.
    unsigned long long edit_epoch;
    bool full_width_space;
};
static_assert(std::is_standard_layout<RustServiceState>::value);
class RustServiceStateStorage final {
public:
    RustServiceStateStorage();
    ~RustServiceStateStorage();
    RustServiceStateStorage(const RustServiceStateStorage&)=delete;
    RustServiceStateStorage& operator=(const RustServiceStateStorage&)=delete;
    RustServiceState* const value;
};
} // namespace meltype
#endif

