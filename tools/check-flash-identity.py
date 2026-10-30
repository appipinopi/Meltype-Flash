#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fail if an upstream sync changes the fork back to C# product entry points."""
import argparse, re, sys, tomllib
from pathlib import Path

UPSTREAM_WORKFLOWS = ('build.yml', 'mac.yml', 'linux.yml', 'winget.yml', 'dotnet-update.yml', 'web.yml', 'ime-tests.yml')
REQUIRED = (
    'README.md', '.gitattributes', 'Cargo.toml', 'rust/settings-app/Cargo.toml',
    'rust/native/src/tsf/identity.rs', 'rust/Manage-Windows.ps1', 'rust/core/src/storage.rs',
    '.github/workflows/rust-release.yml', 'rust/build-windows.ps1',
    '.github/workflows/web.yml',
    *('docs/ci-legacy/'+name for name in UPSTREAM_WORKFLOWS if name != 'web.yml'),
)
def inspect(root):
    errors=[]; files={}
    for name in REQUIRED:
        path=root/name
        if not path.is_file(): errors.append('Missing Flash product file: '+name)
        else: files[name]=path.read_text(encoding='utf-8-sig')
    if errors: return errors
    def require(condition, message):
        if not condition: errors.append(message)
    readme=files['README.md']
    require(readme.startswith('# Meltype-flash\n'), 'Root README must identify Meltype-flash first')
    require('cargo run -p meltype-settings' in readme and 'meltype-flash.exe' in readme, 'Root README must launch the Rust Flash executable')
    require('github.com/yksr-melt/Meltype/releases' not in readme, 'Root README must not offer upstream C# releases as Flash downloads')
    require('src/** linguist-vendored=true' in files['.gitattributes'], 'Upstream C# reference sources must be marked as reference/vendor code')
    try:
        workspace=tomllib.loads(files['Cargo.toml'])
        settings=tomllib.loads(files['rust/settings-app/Cargo.toml'])
        require('rust/settings-app' in workspace['workspace']['members'], 'Rust settings app must remain in the workspace')
        require(any(binary.get('name')=='meltype-flash' for binary in settings.get('bin', [])), 'Settings executable must remain meltype-flash')
    except (tomllib.TOMLDecodeError, KeyError) as error: errors.append('Invalid Rust product manifest: '+str(error))
    require('0xc540baf6_88b1_5b30_b524_2c8702b034fc' in files['rust/native/src/tsf/identity.rs'], 'Flash must keep its independent COM service ID')
    require("$clsid = '{C540BAF6-88B1-5B30-B524-2C8702B034FC}'" in files['rust/Manage-Windows.ps1'], 'Flash installer must not target upstream C# registration')
    require('.join("Meltype-flash")' in files['rust/core/src/storage.rs'], 'Flash default user data directory must stay independent')
    build=files['rust/build-windows.ps1']
    require('cargo build' in build and '-RustCore' in build and 'dotnet publish' not in build, 'Windows Flash build must use Rust/TSF')
    release=files['.github/workflows/rust-release.yml']
    require('  workflow_dispatch:' in release and not re.search(r'^  push:', release, re.M), 'Flash publication must be explicit, never an upstream-sync/push side effect')
    require('flash-v' in release and 'refs/heads/rust' in release, 'Flash releases must use Flash tags from the product branch')
    for name in UPSTREAM_WORKFLOWS:
        active = root/'.github/workflows'/name
        archived = root/'docs/ci-legacy'/name
        paths = [path for path in (active, archived) if path.is_file()]
        for path in paths:
            check_upstream_workflow(path.read_text(encoding='utf-8-sig'), name, errors)
    return errors

def check_upstream_workflow(workflow, name, errors):
    def require(condition, message):
        if not condition: errors.append(message)
    if 'jobs:\n' not in workflow:
        errors.append(name+': missing jobs mapping'); return
    jobs=workflow.split('jobs:\n',1)[1]
    blocks=re.split(r'^  [A-Za-z0-9_-]+:[ \t]*$', jobs, flags=re.M)[1:]
    require(bool(blocks), name+': no jobs found')
    for number, block in enumerate(blocks, 1):
        require(bool(re.search(r"^    if:.*github\.repository == 'yksr-melt/Meltype'", block, re.M)), f'{name} job {number}: upstream C# automation must not run as Flash')
if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('root',type=Path,nargs='?',default=Path(__file__).resolve().parent.parent)
    args=parser.parse_args(); errors=inspect(args.root)
    for error in errors: print('ERROR:',error,file=sys.stderr)
    if errors: raise SystemExit(1)
    print('PASS: Meltype-flash entry points, Rust binary, independent registration/data and release guards')
