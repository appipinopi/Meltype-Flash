// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
fn main() {
    #[cfg(feature = "engine")]
    {
        let target = std::env::var("TARGET").unwrap();
        println!("cargo:rerun-if-changed=vendor/shunti");
        println!("cargo:rerun-if-changed=android-cpu.h");
        let mut engine = cc::Build::new();
        engine
            .cpp(true)
            .std("c++17")
            .opt_level(2)
            .warnings(false)
            .flag_if_supported("/utf-8")
            .file("vendor/shunti/engine.cpp");
        if target == "x86_64-linux-android" {
            engine.flag("-include").flag("android-cpu.h");
        }
        if target.contains("msvc") && (target.starts_with("x86_64") || target.starts_with("i686")) {
            cc::Build::new()
                .cpp(true)
                .std("c++17")
                .opt_level(2)
                .warnings(false)
                .flag("/arch:AVX2")
                .flag("/utf-8")
                .file("vendor/shunti/engine_avx2.cpp")
                .compile("shunti_avx2");
        }
        engine.compile("meltype_shunti");
    }
}
