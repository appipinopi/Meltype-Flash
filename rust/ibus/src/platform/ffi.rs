// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
//! Opaque C ABI for libibus and GLib. GType queries supply object/class sizes;
//! no private IBus structures, copied vtables or C/C++ shim are required.
#![allow(non_camel_case_types)]
use std::ffi::{c_char, c_int, c_void};
pub type Obj = *mut c_void;
#[repr(C)]
#[derive(Default)]
pub struct GTypeQuery {
    pub type_: usize,
    pub name: *const c_char,
    pub class_size: u32,
    pub instance_size: u32,
}
#[link(name = "gobject-2.0")]
extern "C" {
    pub fn g_object_new(type_: usize, first_property: *const c_char, ...) -> Obj;
    pub fn g_type_class_ref(type_: usize) -> Obj;
    pub fn g_type_class_unref(class: Obj);
    pub fn g_object_class_find_property(class: Obj, name: *const c_char) -> Obj;
    pub fn g_signal_lookup(name: *const c_char, type_: usize) -> u32;
    pub fn g_type_query(type_: usize, query: *mut GTypeQuery);
    pub fn g_type_register_static_simple(
        parent: usize,
        name: *const c_char,
        class_size: u32,
        class_init: Option<unsafe extern "C" fn(Obj, Obj)>,
        instance_size: u32,
        instance_init: Option<unsafe extern "C" fn(Obj, Obj)>,
        flags: u32,
    ) -> usize;
    pub fn g_signal_connect_data(
        instance: Obj,
        name: *const c_char,
        callback: *const c_void,
        data: Obj,
        destroy: Option<unsafe extern "C" fn(Obj, Obj)>,
        flags: u32,
    ) -> usize;
    pub fn g_object_set_data_full(
        object: Obj,
        key: *const c_char,
        data: Obj,
        destroy: Option<unsafe extern "C" fn(Obj)>,
    );
    pub fn g_object_get_data(object: Obj, key: *const c_char) -> Obj;
    pub fn g_object_ref(object: Obj) -> Obj;
    pub fn g_object_ref_sink(object: Obj) -> Obj;
    pub fn g_object_unref(object: Obj);
}
#[link(name = "glib-2.0")]
extern "C" {
    pub fn g_main_loop_new(context: Obj, running: c_int) -> Obj;
    pub fn g_main_loop_run(loop_: Obj);
    pub fn g_main_loop_quit(loop_: Obj);
    pub fn g_main_loop_unref(loop_: Obj);
    pub fn g_timeout_add_full(
        priority: c_int,
        interval: u32,
        callback: Option<unsafe extern "C" fn(Obj) -> c_int>,
        data: Obj,
        destroy: Option<unsafe extern "C" fn(Obj)>,
    ) -> u32;
    pub fn g_source_remove(id: u32) -> c_int;
}
#[link(name = "ibus-1.0")]
extern "C" {
    pub fn ibus_init();
    pub fn ibus_service_get_connection(service: Obj) -> Obj;
    pub fn ibus_bus_new() -> Obj;
    pub fn ibus_bus_is_connected(bus: Obj) -> c_int;
    pub fn ibus_bus_get_connection(bus: Obj) -> Obj;
    pub fn ibus_bus_request_name(bus: Obj, name: *const c_char, flags: u32) -> u32;
    pub fn ibus_bus_register_component(bus: Obj, component: Obj) -> c_int;
    pub fn ibus_factory_new(connection: Obj) -> Obj;
    pub fn ibus_factory_add_engine(factory: Obj, name: *const c_char, type_: usize);
    pub fn ibus_component_new(
        name: *const c_char,
        description: *const c_char,
        version: *const c_char,
        license: *const c_char,
        author: *const c_char,
        homepage: *const c_char,
        command: *const c_char,
        textdomain: *const c_char,
    ) -> Obj;
    pub fn ibus_component_add_engine(component: Obj, engine: Obj);
    pub fn ibus_engine_desc_new(
        name: *const c_char,
        longname: *const c_char,
        description: *const c_char,
        language: *const c_char,
        license: *const c_char,
        author: *const c_char,
        icon: *const c_char,
        layout: *const c_char,
    ) -> Obj;
    pub fn ibus_engine_get_type() -> usize;
    pub fn ibus_keyval_to_unicode(keyval: u32) -> u32;
    pub fn ibus_text_new_from_string(text: *const c_char) -> Obj;
    pub fn ibus_text_get_text(text: Obj) -> *const c_char;
    pub fn ibus_text_append_attribute(text: Obj, type_: u32, value: u32, start: u32, end: i32);
    pub fn ibus_engine_get_surrounding_text(
        engine: Obj,
        text: *mut Obj,
        cursor: *mut u32,
        anchor: *mut u32,
    );
    pub fn ibus_engine_delete_surrounding_text(engine: Obj, offset: i32, count: u32);
    pub fn ibus_engine_commit_text(engine: Obj, text: Obj);
    pub fn ibus_engine_forward_key_event(engine: Obj, keyval: u32, keycode: u32, state: u32);
    pub fn ibus_engine_update_preedit_text_with_mode(
        engine: Obj,
        text: Obj,
        cursor: u32,
        visible: c_int,
        mode: c_int,
    );
    pub fn ibus_engine_update_auxiliary_text(engine: Obj, text: Obj, visible: c_int);
    pub fn ibus_engine_hide_auxiliary_text(engine: Obj);
    pub fn ibus_engine_update_lookup_table(engine: Obj, table: Obj, visible: c_int);
    pub fn ibus_engine_hide_lookup_table(engine: Obj);
    pub fn ibus_lookup_table_new(
        page_size: u32,
        cursor: u32,
        cursor_visible: c_int,
        round: c_int,
    ) -> Obj;
    pub fn ibus_lookup_table_append_candidate(table: Obj, text: Obj);
    pub fn ibus_lookup_table_set_orientation(table: Obj, orientation: c_int);
    pub fn ibus_prop_list_new() -> Obj;
    pub fn ibus_prop_list_append(list: Obj, property: Obj);
    pub fn ibus_property_new(
        key: *const c_char,
        type_: c_int,
        label: Obj,
        icon: *const c_char,
        tooltip: Obj,
        sensitive: c_int,
        visible: c_int,
        state: c_int,
        sub_props: Obj,
    ) -> Obj;
    pub fn ibus_engine_register_properties(engine: Obj, properties: Obj);
}
