#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""Build a signed CRX3 package and Chrome update manifest for the ChromeOS
extension. Chrome checks the update manifest at the extension's update_url and
then downloads the CRX; the extension ID comes from the signing key, so the
same key must be used for every release."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zipfile
from xml.sax.saxutils import escape

SIGNATURE_CONTEXT = b"CRX3 SignedData\x00"


def varint(value: int) -> bytes:
    output = bytearray()
    while True:
        part = value & 0x7F
        value >>= 7
        if value:
            output.append(part | 0x80)
        else:
            output.append(part)
            return bytes(output)


def field(number: int, payload: bytes) -> bytes:
    return varint((number << 3) | 2) + varint(len(payload)) + payload


def crx_id(public_key: bytes) -> str:
    """Chrome's 32-character extension ID (a-p) for a DER public key."""
    digest = hashlib.sha256(public_key).digest()[:16]
    return "".join(chr(ord("a") + (byte >> 4)) + chr(ord("a") + (byte & 0x0F)) for byte in digest)


def build_crx(archive: bytes, public_key: bytes, signature: bytes) -> bytes:
    signed_data = field(1, hashlib.sha256(public_key).digest()[:16])
    proof = field(1, public_key) + field(2, signature)
    header = field(2, proof) + field(10000, signed_data)
    return b"Cr24" + struct.pack("<II", 3, len(header)) + header + archive


def signature_payload(signed_data: bytes, archive: bytes) -> bytes:
    return SIGNATURE_CONTEXT + struct.pack("<I", len(signed_data)) + signed_data + archive


def openssl(*args, data: bytes | None = None) -> bytes:
    return subprocess.run(
        ["openssl", *args],
        input=data,
        check=True,
        capture_output=True,
    ).stdout


def public_key(key: Path) -> bytes:
    return openssl("rsa", "-in", str(key), "-pubout", "-outform", "DER")


def sign(key: Path, data: bytes) -> bytes:
    return openssl("dgst", "-sha256", "-sign", str(key), data=data)


def update_manifest(app_id: str, codebase: str, version: str) -> bytes:
    return (
        "<?xml version='1.0' encoding='UTF-8'?>\n"
        "<gupdate xmlns='http://www.google.com/update2/response' protocol='2.0'>\n"
        f"  <app appid='{escape(app_id)}'>\n"
        f"    <updatecheck codebase='{escape(codebase)}' version='{escape(version)}' />\n"
        "  </app>\n"
        "</gupdate>\n"
    ).encode("utf-8")


def manifest_version(archive: Path) -> str:
    with zipfile.ZipFile(archive) as package:
        manifest = json.loads(package.read("manifest.json"))
    version = manifest.get("version")
    if not isinstance(version, str) or not version:
        raise SystemExit("ChromeOS ZIP の manifest.json に version がありません")
    return version


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--zip", type=Path, required=True, help="Meltype-flash-ChromeOS.zip")
    parser.add_argument("--key", type=Path, required=True, help="RSA private key (PEM)")
    parser.add_argument("--version", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--base-url",
        default="https://github.com/appipinopi/Meltype-Flash/releases/download",
    )
    args = parser.parse_args()
    if manifest_version(args.zip) != args.version:
        raise SystemExit("--version と ZIP 内の manifest.json の版が一致しません")
    archive = args.zip.read_bytes()
    key = public_key(args.key)
    signed_data = field(1, hashlib.sha256(key).digest()[:16])
    crx = build_crx(archive, key, sign(args.key, signature_payload(signed_data, archive)))
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "Meltype-flash-ChromeOS.crx").write_bytes(crx)
    codebase = f"{args.base_url.rstrip('/')}/{args.tag}/Meltype-flash-ChromeOS.crx"
    (args.output / "updates.xml").write_bytes(update_manifest(crx_id(key), codebase, args.version))
    print(json.dumps({"extension_id": crx_id(key), "version": args.version, "crx_bytes": len(crx)}))


if __name__ == "__main__":
    main()
