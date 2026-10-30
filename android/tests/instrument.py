# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Run a fixed Android test class and reject empty/skipped/failed test reports."""
import argparse
from pathlib import Path
import re
import subprocess


def verify_report(output: str, expected: int) -> None:
    statuses = [int(value) for value in re.findall(r"^INSTRUMENTATION_STATUS_CODE: (-?\d+)\s*$", output, re.M)]
    if (statuses.count(0) != expected or any(value not in (0, 1, 2) for value in statuses)
            or not re.search(rf"OK \({expected} tests?\)", output)
            or not re.search(r"^INSTRUMENTATION_CODE: -1\s*$", output, re.M)
            or "FAILURES!!!" in output or "INSTRUMENTATION_FAILED" in output):
        raise RuntimeError(f"Expected {expected} completed Android tests without failures/skips")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--adb", required=True)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"emulator-\d+", args.serial):
        parser.error("Only an explicitly selected emulator is supported")
    args.output.mkdir(parents=True, exist_ok=True)
    adb = [args.adb, "-s", args.serial]
    command = [
        *adb, "shell", "am", "instrument", "-w", "-r", "-e", "class",
        "moe.meltype.rust.NativeAndroidTest",
        "moe.meltype.rust.test/androidx.test.runner.AndroidJUnitRunner",
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=300)
    except subprocess.TimeoutExpired as error:
        partial = error.stdout or b""
        if isinstance(partial, bytes):
            partial = partial.decode("utf-8", errors="replace")
        (args.output / "instrumentation.txt").write_text(partial, encoding="utf-8")
        raise
    report = result.stdout + result.stderr
    (args.output / "instrumentation.txt").write_text(report, encoding="utf-8")
    print(report, flush=True)
    # Retrieve only this test app's own generated preview/metadata.
    for name in ("flick-view.png", "input-view.png", "input-view-narrow.png", "input-view-large-text.png", "settings-page.png", "settings-activity.png", "learning-confirmation.png", "restore-confirmation.png", "device.txt", "label-fit.txt"):
        artifact = subprocess.run([
            *adb, "exec-out", "run-as", "moe.meltype.rust", "cat",
            f"files/instrumentation-artifacts/{name}",
        ], capture_output=True, timeout=30)
        if artifact.returncode == 0:
            (args.output / name).write_bytes(artifact.stdout)
    result.check_returncode()
    verify_report(report, 14)
    for name in ("flick-view.png", "input-view.png", "input-view-narrow.png", "input-view-large-text.png", "settings-page.png", "settings-activity.png", "learning-confirmation.png", "restore-confirmation.png"):
        if not (args.output / name).read_bytes().startswith(b"\x89PNG\r\n\x1a\n"):
            raise RuntimeError(f"Android preview PNG is missing or invalid: {name}")
    for name in ("device.txt", "label-fit.txt"):
        if not (args.output / name).is_file():
            raise RuntimeError(f"Android runtime metadata is missing: {name}")


if __name__ == "__main__":
    main()
