// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Android x86_64: avoid the NDK's non-PIC __cpu_model dispatch reference.
// Keep the pinned upstream file unchanged and preserve the OS AVX-state check.
#pragma once
#include <cpuid.h>
static inline bool meltype_android_shunti_avx2_fma() {
    unsigned a, b, c, d;
    if (__get_cpuid_max(0, nullptr) < 7 || !__get_cpuid(1, &a, &b, &c, &d)) return false;
    constexpr unsigned required = (1u << 12) | (1u << 27) | (1u << 28);
    if ((c & required) != required) return false;
    unsigned state_low, state_high;
    __asm__ volatile("xgetbv" : "=a"(state_low), "=d"(state_high) : "c"(0));
    if ((state_low & 6) != 6) return false;
    __cpuid_count(7, 0, a, b, c, d);
    return (b & (1u << 5)) != 0;
}
#define __builtin_cpu_init() ((void)0)
#define __builtin_cpu_supports(feature) meltype_android_shunti_avx2_fma()
