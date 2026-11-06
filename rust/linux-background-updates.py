#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""ユーザー領域の更新タイマー。systemd非対応ではデスクトップのログイン時に確認する。"""
import json
import datetime
import os
from pathlib import Path
import re
import subprocess
import sys
import time

MARKER = '# Meltype-FLASH managed updates\n'
NAME = 'meltype-flash-update'


def systemctl(*args):
    try:
        return subprocess.run(['systemctl', '--user', *args], capture_output=True, timeout=5).returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


def managed(path, text=None):
    if path.exists() and not path.read_text(encoding='utf-8').startswith(MARKER):
        raise ValueError('同名の管理対象外ファイルがあるため更新設定を変更できません: ' + str(path))
    if text is None:
        path.unlink(missing_ok=True)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(MARKER + text, encoding='utf-8')


def quote(value, desktop=False):
    text = str(value).replace('\\', '\\\\').replace('"', '\\"').replace('%', '%%')
    if desktop:
        text = text.replace('`', '\\`').replace('$', '\\$')
    else:
        text = text.replace('$', '$$')
    text = '"' + text + '"'
    return text.replace('\\', '\\\\') if desktop else text


def configure(root, binary, version, enabled):
    config = Path(os.environ.get('XDG_CONFIG_HOME', Path.home() / '.config'))
    service = config / 'systemd/user' / (NAME + '.service')
    timer = service.with_suffix('.timer')
    desktop = config / 'autostart' / (NAME + '.desktop')
    if not enabled:
        systemctl('disable', '--now', NAME + '.timer')
        for path in (desktop, timer, service):
            managed(path)
        systemctl('daemon-reload')
        return 'disabled'
    args = [sys.executable, str(Path(__file__).resolve()), 'run', str(root), str(binary), version]
    managed(service, '[Unit]\nDescription=Meltype-FLASH update check\n[Service]\nType=oneshot\nExecStart=' + ' '.join(quote(arg) for arg in args) + '\n')
    managed(timer, '[Unit]\nDescription=Meltype-FLASH daily update check\n[Timer]\nOnStartupSec=5min\nOnUnitActiveSec=1d\nOnCalendar=daily\nPersistent=true\n[Install]\nWantedBy=timers.target\n')
    if systemctl('daemon-reload') and systemctl('enable', '--now', NAME + '.timer'):
        managed(desktop)
        return 'daily'
    systemctl('disable', '--now', NAME + '.timer')
    managed(desktop, '[Desktop Entry]\nType=Application\nName=Meltype-FLASH 更新確認\nNoDisplay=true\nTerminal=false\nExec=' + ' '.join(quote(arg, True) for arg in args) + '\n')
    return 'login'


def run(root, binary, fallback):
    settings = json.loads((root / 'config.json').read_text(encoding='utf-8'))
    if not settings.get('AutoUpdate', False):
        return
    import fcntl
    with (root / '.update-check.lock').open('w') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return
        stamp = root / '.last-update-check'
        try:
            now = time.time()
            previous = float(stamp.read_text())
            if previous <= now and datetime.date.fromtimestamp(previous) == datetime.date.fromtimestamp(now):
                return
        except (OSError, ValueError, OverflowError):
            pass
        version = fallback
        choices = [binary / 'meltype-rs', Path('/usr/libexec/meltype-flash/ibus/meltype-rs'), Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share')) / 'Meltype-flash/ibus/meltype-rs']
        cli = next((path for path in choices if path.is_file()), choices[0])
        if cli.is_file():
            output = subprocess.check_output([str(cli), '--version'], text=True, timeout=5)
            found = re.search(r'\d+\.\d+\.\d+', output)
            if found:
                version = found[0]
        subprocess.run([sys.executable, str(root / 'update-linux.py'), 'apply', version], check=True)
        stamp.write_text(str(time.time()))


if __name__ == '__main__':
    try:
        action, root, binary, version = sys.argv[1:5]
        root, binary = Path(root).resolve(), Path(binary).resolve()
        if action == 'run':
            run(root, binary, version)
        elif action in ('enable', 'disable'):
            print(configure(root, binary, version, action == 'enable'))
        else:
            raise ValueError('Unknown action')
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
