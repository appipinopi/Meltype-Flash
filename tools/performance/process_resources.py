# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""POSIX child CPU/RSS accounting. Missing Windows-only metrics stay null."""
import os
from pathlib import Path
import subprocess
import sys
import time


def physical_memory():
    if sys.platform == "darwin":
        return int(subprocess.check_output(["sysctl", "-n", "hw.memsize"]))
    return os.sysconf("SC_PHYS_PAGES") * os.sysconf("SC_PAGE_SIZE")


def measure_process(command, *, cwd, env, stdout):
    total = physical_memory()
    logical = os.cpu_count() or 1
    samples = []
    started = time.perf_counter()
    process = subprocess.Popen(command, cwd=cwd, env=env, stdout=stdout)
    try:
        while True:
            pid, status, usage = os.wait4(process.pid, os.WNOHANG)
            if pid:
                process.returncode = os.waitstatus_to_exitcode(status)
                break
            if sys.platform.startswith("linux"):
                try:
                    for line in Path(f"/proc/{process.pid}/status").read_text().splitlines():
                        if line.startswith("VmRSS:"):
                            samples.append(int(line.split()[1]) * 1024)
                except FileNotFoundError:
                    pass
            time.sleep(.05)
    finally:
        if process.returncode is None:
            process.kill()
            process.wait()
    elapsed = time.perf_counter() - started
    if process.returncode:
        raise subprocess.CalledProcessError(process.returncode, command)
    cpu = usage.ru_utime + usage.ru_stime
    peak = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    return dict(wall_seconds=elapsed, cpu_seconds=cpu,
                cpu_percent_one_core=cpu / elapsed * 100,
                cpu_percent_machine=cpu / elapsed / logical * 100,
                logical_processors=logical, peak_working_set_bytes=peak,
                sampled_mean_working_set_bytes=sum(samples) / len(samples) if samples else None,
                sampled_peak_private_bytes=None, peak_commit_bytes=None,
                peak_working_set_percent_physical=peak / total * 100,
                memory_sample_count=len(samples), poll_interval_seconds=.05,
                system_memory_before=dict(total_bytes=total), system_memory_after=None,
                scope="Whole child process, wait4 user+system CPU and kernel peak RSS. "
                      "Linux mean RSS sampled at 50ms; macOS mean/private memory unavailable. "
                      "Includes startup, sessions, warmups and JSON output; not whole IME.")
