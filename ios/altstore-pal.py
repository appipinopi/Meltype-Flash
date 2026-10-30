#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""署名済みアプリと公証後のADPから、公開前のPALソースJSONを作る。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import plistlib
import re
import subprocess
from urllib.parse import urlsplit


def https_url(value):
    parsed = urlsplit(value)
    if parsed.scheme != 'https' or not parsed.hostname or parsed.username or parsed.password:
        raise ValueError('公開URLには認証情報を含まないHTTPS URLを指定してください')
    if parsed.fragment:
        raise ValueError('公開URLにフラグメントは指定できません')
    return value


def signed_entitlements(bundle):
    # codesignはmacOSで実行する。署名の検査はAppleの公証確認を代替しない。
    subprocess.run(['codesign', '--verify', '--strict', str(bundle)], check=True,
                   capture_output=True)
    result = subprocess.run(['codesign', '--display', '--entitlements', ':-', str(bundle)],
                            check=True, capture_output=True)
    entitlements = plistlib.loads(result.stdout)
    if entitlements.get('get-task-allow'):
        raise ValueError('開発用の署名はPAL配布に使用できません')
    if not entitlements.get('application-identifier') or not entitlements.get('com.apple.developer.team-identifier'):
        raise ValueError('Appleの配布署名とTeam IDが必要です')
    return entitlements


def make_source(app, adp, *, tag, marketplace_id, developer_name, icon_url,
                download_url, release_date):
    match = re.fullmatch(r'flash-v(\d+\.\d+\.\d+)-preview\.(\d+)', tag)
    if not match or (*map(int, match[1].split('.')), int(match[2])) < (0, 1, 0, 6):
        raise ValueError('PAL対応はflash-v0.1.0-preview.6以降のプレビューが対象です')
    if not re.fullmatch(r'[1-9]\d*', marketplace_id):
        raise ValueError('App Store Connectの実際のApple IDを指定してください')
    if not developer_name.strip():
        raise ValueError('開発者の表示名が必要です')
    datetime.fromisoformat(release_date.replace('Z', '+00:00'))
    https_url(icon_url)
    https_url(download_url)
    if not urlsplit(download_url).path.endswith('/manifest.json'):
        raise ValueError('IPAではなく、公開するADPのmanifest.json URLを指定してください')
    app, adp = Path(app), Path(adp)
    if not (adp / 'signature').is_file() or not (adp / 'signature').stat().st_size:
        raise ValueError('公証後のADPのsignatureが必要です')
    manifest = json.loads((adp / 'manifest.json').read_text(encoding='utf-8'))
    if not isinstance(manifest, dict) or not manifest:
        raise ValueError('ADPのmanifest.jsonが空です')
    variants = sorted((adp / 'variant').rglob('*.ipa'))
    if not variants or any(p.stat().st_size == 0 for p in variants):
        raise ValueError('ADPのvariant内に実際のIPAが必要です')
    info = plistlib.loads((app / 'Info.plist').read_bytes())
    if info.get('CFBundleShortVersionString') != match[1]:
        raise ValueError('タグとアプリのCFBundleShortVersionStringが一致しません')
    if not re.fullmatch(r'\d+(?:\.\d+){0,2}', str(info.get('CFBundleVersion', ''))):
        raise ValueError('CFBundleVersionが必要です')
    if not info.get('MinimumOSVersion') or not info.get('CFBundleIdentifier'):
        raise ValueError('アプリの最低OSとBundle Identifierが必要です')
    keyboard = app / 'PlugIns' / 'MeltypeFlashKeyboard.appex'
    if not keyboard.is_dir():
        raise ValueError('キーボード拡張がありません。拡張を保持して署名してください')
    permissions, privacy = set(), {}
    parent_entitlements = None
    bundles = [app, *sorted(app.rglob('*.appex'))]
    for bundle in bundles:
        bundle_info = plistlib.loads((bundle / 'Info.plist').read_bytes())
        if bundle_info.get('DTPlatformName') != 'iphoneos':
            raise ValueError('Simulator版はPAL配布に使用できません')
        for key in ('CFBundleShortVersionString', 'CFBundleVersion'):
            if bundle_info.get(key) != info[key]:
                raise ValueError('本体と拡張のバージョンが一致しません')
        if bundle == keyboard:
            if not bundle_info['CFBundleIdentifier'].startswith(info['CFBundleIdentifier'] + '.'):
                raise ValueError('キーボードのBundle Identifierが本体に属していません')
            extension = bundle_info.get('NSExtension', {})
            if extension.get('NSExtensionPointIdentifier') != 'com.apple.keyboard-service':
                raise ValueError('キーボード拡張の種類が不正です')
        entitlements = signed_entitlements(bundle)
        if not entitlements.get('com.apple.security.application-groups'):
            raise ValueError('背景設定を共有するApp Groupの配布署名が必要です')
        team = entitlements['com.apple.developer.team-identifier']
        if entitlements['application-identifier'] != team + '.' + bundle_info['CFBundleIdentifier']:
            raise ValueError('署名のApplication IdentifierとBundle Identifierが一致しません')
        if parent_entitlements is None:
            parent_entitlements = entitlements
        elif (team != parent_entitlements['com.apple.developer.team-identifier'] or
              set(entitlements.get('com.apple.security.application-groups', [])) !=
              set(parent_entitlements.get('com.apple.security.application-groups', []))):
            raise ValueError('本体と拡張の署名チーム・App Groupが一致しません')
        permissions.update(entitlements)
        privacy.update({key: value for key, value in bundle_info.items()
                        if key.startswith('NS') and 'UsageDescription' in key})
    permissions.difference_update({'application-identifier', 'com.apple.developer.team-identifier'})
    return {
        'name': 'Meltype-flash',
        'subtitle': '日本語・英語を入力できるキーボード',
        'website': 'https://github.com/appipinopi/Meltype-Flash',
        'nsfw': False,
        'apps': [{
            'name': 'Meltype-flash',
            'bundleIdentifier': info['CFBundleIdentifier'],
            'marketplaceID': marketplace_id,
            'developerName': developer_name,
            'localizedDescription': '日本語・英語の入力、かなフリックとQWERTYに対応するキーボードです。設定アプリでキーボードを追加して使います。',
            'iconURL': icon_url,
            'category': 'utilities',
            'versions': [{
                'version': info['CFBundleShortVersionString'],
                'buildVersion': str(info['CFBundleVersion']),
                'marketingVersion': f'{match[1]} Pre{match[2]}',
                'date': release_date,
                'downloadURL': download_url,
                'size': variants[0].stat().st_size,
                'minOSVersion': info['MinimumOSVersion'],
            }],
            'appPermissions': {'entitlements': sorted(permissions), 'privacy': privacy},
        }],
        'news': [],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, required=True, help='公証に提出した署名済み.app')
    parser.add_argument('--adp', type=Path, required=True, help='公証後に取得・展開したADPのルート')
    for name in ('tag', 'marketplace-id', 'developer-name', 'icon-url', 'download-url', 'release-date'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = vars(parser.parse_args())
    output = args.pop('output')
    try:
        source = make_source(**args)
        output.parent.mkdir(parents=True, exist_ok=True)
        # 既存の公開履歴やソースを誤って上書きしない。
        with output.open('x', encoding='utf-8') as stream:
            json.dump(source, stream, ensure_ascii=False, indent=2)
            stream.write('\n')
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'PALソースを生成できません: {error}\n')


if __name__ == '__main__':
    main()
