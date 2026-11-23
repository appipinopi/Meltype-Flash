# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Run actual JNI against a private JVM editor; no Android device is modified."""
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[3]
library = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="meltype-jni-host-") as output:
    generator = Path(output) / ("bridge.exe" if sys.platform == "win32" else "bridge")
    subprocess.run(["rustc", "--edition=2021", str(root / "android/bridge.rs"), "-o", str(generator)], check=True)
    generated = Path(output) / "generated"
    subprocess.run([str(generator), str(generated)], check=True)
    adapters = generated / "moe/meltype/rust"
    subprocess.run([
        "javac", "-encoding", "UTF-8", "--release", "17", "-d", output,
        *(str(adapters / name) for name in ("Native.java", "Events.java", "FlickKeyboard.java", "FlickGesture.java")),
        str(root / "rust/native/tests/android/FlickHost.java"),
        str(root / "rust/native/tests/android/EditorHost.java"),
        str(root / "rust/native/tests/android/InputHost.java"),
        str(root / "rust/native/tests/android/ContextHost.java"),
        str(root / "rust/native/tests/android/KeyEvent.java"),
        str(root / "rust/native/tests/android/LifecycleHost.java"),
        str(root / "rust/native/tests/android/StateHost.java"),
        str(root / "rust/native/tests/android/WidgetHost.java"),
        str(root / "rust/native/tests/android/DispatchHost.java"),
        str(root / "rust/native/tests/android/SettingsHost.java"),
        str(root / "rust/native/tests/android/DocumentHost.java"),
        str(root / "rust/native/tests/android/SettingsViewHost.java"),
        str(root / "rust/native/tests/android/ActivityViewHost.java"),
        str(root / "rust/native/tests/android/DialogHost.java"),
        str(root / "rust/native/tests/android/SettingsUiHost.java"),
        str(root / "rust/native/tests/android/ServiceHost.java"),
        str(root / "rust/native/tests/android/SettingsPage.java"),
        str(root / "rust/native/tests/android/SettingsActivity.java"),
        *(str(path) for path in sorted((root / "rust/native/tests/android/widgets").glob("*.java"))),
    ], check=True)
    for host in sys.argv[2:] or ["FlickHost", "ServiceHost", "SettingsUiHost", "DialogHost", "ActivityViewHost", "SettingsViewHost", "DocumentHost", "SettingsHost", "DispatchHost", "WidgetHost", "StateHost", "LifecycleHost", "ContextHost", "InputHost", "EditorHost"]:
        subprocess.run([
            "java", "-Xcheck:jni", f"-Djava.library.path={library}", "-cp", output,
            f"moe.meltype.rust.{host}",
        ], check=True)
