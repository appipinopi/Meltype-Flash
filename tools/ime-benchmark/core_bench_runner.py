#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""第2弾: 1万文コア比較ベンチの 1 対象ぶんの測定。

`core_targets.json` の 1 対象について、

  1. GitHub から対象リポジトリを浅く取得 (ソースは書き換えない)
  2. 種別ごとに測定用ハーネスを組み立ててビルド
       csharp   … src/Meltype.Core を参照する CoreBench (tools/core-bench/)
       rust     … rust/core/examples/core_bench.rs を差し込んでビルド
       monakime … リポジトリ同梱の monakime-cli --stdin
       kswime   … リポジトリ同梱の kswime (標準入力の行ごと変換)
  3. 1万文コーパスを打鍵して測定
       Layer A (変換器なし): ローマ字 → かな・英日境界・保護の結果
       Layer B (共通 Mozc) : Space で変換した表示と Enter で確定した文字列
  4. 結果を JSON に書く (失敗しても終了コード 0。理由とログを JSON に残す)

  python tools/ime-benchmark/core_bench_runner.py --target upstream_v171 --out bench-out/upstream-v1.7.1.json

環境変数:
  LIMIT                  先頭 N 件だけ測る (空=全 1 万件)
  CORE_BENCH_MAX_SECONDS 1 対象のソフト期限 (既定 6600 秒 = 110 分。ワークフローの 120 分より短くして、
                         打ち切っても結果 JSON を artifact に残せるようにする)
  CORE_BENCH_MOZC_HELPER Mozc ヘルパーの既存パス (未指定なら本家 v1.7.1 の Linux 配布物から取り出す)
  CORE_BENCH_CACHE       配布物のキャッシュ先 (既定 $RUNNER_TEMP/core-bench-cache)
  CORE_BENCH_WORK        作業ディレクトリ (既定 $RUNNER_TEMP/core-bench-work)
"""
import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time
import traceback
import urllib.request
import zipfile
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TARGETS = HERE / "core_targets.json"
HARNESS = HERE / "harness"
DEFAULT_CORPUS = HERE / "data"
DEFAULT_MAX_SECONDS = 6600

sys.path.insert(0, str(HERE))
from kana2romaji import convert as kana_convert  # noqa: E402


class BenchError(Exception):
    def __init__(self, stage, detail, log=""):
        super().__init__(detail)
        self.stage = stage
        self.detail = detail
        self.log = log


# ---------------------------------------------------------------- 入力

def load_targets(path=TARGETS):
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    for target in data["targets"]:
        if not target.get("id"):
            raise BenchError("input", "対象 ID の無い行があります")
    return data


def find_target(data, target_id):
    for target in data["targets"]:
        if target["id"] == target_id or target["name"] == target_id:
            return target
    known = ", ".join(t["id"] for t in data["targets"])
    raise BenchError("input", f"対象 {target_id!r} は core_targets.json にありません (指定できる ID: {known})")


def load_cases(corpus=DEFAULT_CORPUS, limit=None):
    """コーパスを読み、打鍵するローマ字とかなの期待値を作る。"""
    parts = sorted(Path(corpus).glob("*.jsonl"))
    if not parts:
        raise BenchError("corpus", f"コーパスがありません: {corpus}")
    rows = []
    for part in parts:
        with part.open(encoding="utf-8") as handle:
            rows.extend(json.loads(line) for line in handle if line.strip())
    rows.sort(key=lambda row: row["id"])
    if limit:
        rows = rows[: int(limit)]
    cases = []
    for row in rows:
        typed, kana_target = kana_convert(row["input"])
        cases.append(
            {
                "id": row["id"],
                "category": row.get("category", ""),
                "typed": typed,
                "kana": kana_target,
                "expected": row["expected"],
            }
        )
    return cases


def write_cases(path, cases):
    with Path(path).open("w", encoding="utf-8") as handle:
        for case in cases:
            handle.write(json.dumps({"id": case["id"], "typed": case["typed"]}, ensure_ascii=False) + "\n")


# ---------------------------------------------------------------- 実行

class Session:
    """1 対象の実行。作業ディレクトリ・期限・ログをまとめる。"""

    def __init__(self, target, data, work, deadline):
        self.target = target
        self.data = data
        self.work = Path(work)
        self.deadline = deadline
        self.steps = []
        self.logs = []

    def step(self, text):
        self.steps.append(text)
        print(f"[{self.target['id']}] {text}", flush=True)

    def remaining(self):
        return self.deadline - time.time()

    def run(self, cmd, cwd=None, env=None, timeout=600, stdin=None, label=None, check=True):
        left = self.remaining()
        if left <= 0:
            raise BenchError("deadline", "ソフト期限を過ぎました", self.log())
        timeout = max(1, min(timeout, int(left)))
        started = time.time()
        try:
            result = subprocess.run(
                cmd,
                cwd=str(cwd) if cwd else None,
                env=env,
                input=stdin,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
        except FileNotFoundError as error:
            raise BenchError("exec", f"コマンドがありません: {cmd[0]}", str(error)) from error
        except subprocess.TimeoutExpired as error:
            raise BenchError("timeout", f"{label or cmd[0]} が {timeout} 秒で終わりませんでした", self.log()) from error
        self.logs.append(
            {
                "label": label or " ".join(str(part) for part in cmd[:3]),
                "seconds": round(time.time() - started, 2),
                "code": result.returncode,
            }
        )
        if check and result.returncode != 0:
            raise BenchError(
                "command",
                f"{label or cmd[0]} が終了コード {result.returncode} で失敗しました",
                tail(result.stdout + "\n" + result.stderr),
            )
        return result

    def log(self):
        parts = []
        for entry in self.logs[-6:]:
            parts.append(f"$ {entry['label']} -> {entry['code']} ({entry['seconds']}s)")
        return "\n".join(parts)


def tail(text, limit=4000):
    text = (text or "").strip()
    return text if len(text) <= limit else "…\n" + text[-limit:]


def download(url, dest, timeout=600):
    dest = Path(dest)
    if dest.is_file() and dest.stat().st_size > 0:
        return dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    temporary = dest.with_suffix(dest.suffix + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "meltype-core-bench"})
    with urllib.request.urlopen(request, timeout=timeout) as response, temporary.open("wb") as handle:
        shutil.copyfileobj(response, handle)
    temporary.replace(dest)
    return dest


def ensure_mozc(session, cache_dir):
    """共通 Mozc ヘルパー (本家 v1.7.1 の Linux 配布物) を用意する。"""
    existing = os.environ.get("CORE_BENCH_MOZC_HELPER")
    if existing and Path(existing).is_file():
        session.step(f"mozc helper (env): {existing}")
        return Path(existing)
    cache = Path(cache_dir)
    helper = cache / "meltype_mozc_helper"
    if helper.is_file():
        session.step(f"mozc helper (cache): {helper}")
        return helper
    info = session.data["mozc"]
    archive = cache / info["asset"]
    session.step(f"mozc helper を取得: {info['url']}")
    download(info["url"], archive)
    with zipfile.ZipFile(archive) as bundle:
        member = info["member"]
        with bundle.open(member) as source, helper.open("wb") as target:
            shutil.copyfileobj(source, target)
    helper.chmod(0o755)
    session.step(f"mozc helper を展開: {helper}")
    return helper


# ---------------------------------------------------------------- リポジトリ

def clone(session, dest):
    target = session.target
    url = f"https://github.com/{target['repo']}.git"
    ref = target["ref"]
    dest = Path(dest)
    shutil.rmtree(dest, ignore_errors=True)
    session.step(f"clone {target['repo']}@{ref}")
    result = session.run(
        ["git", "clone", "--depth", "1", "--single-branch", "--branch", ref, url, str(dest)],
        timeout=900,
        label="git clone",
        check=False,
    )
    if result.returncode != 0:
        shutil.rmtree(dest, ignore_errors=True)
        session.run(["git", "clone", "--filter=blob:none", "--no-checkout", url, str(dest)], timeout=900, label="git clone (ref)")
        session.run(["git", "-C", str(dest), "fetch", "--depth", "1", "origin", ref], timeout=900, label="git fetch")
        session.run(["git", "-C", str(dest), "checkout", "--force", "FETCH_HEAD"], timeout=300, label="git checkout")
    sha = session.run(["git", "-C", str(dest), "rev-parse", "HEAD"], timeout=60, label="git rev-parse").stdout.strip()
    session.step(f"commit {sha[:12]}")
    return dest, sha


def read_text(path):
    try:
        return Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def detect_generation(root):
    """C# コアの世代をソースから判定する (1: CompositionOptions / 2: CompositionServices / 3: ImeHost)。"""
    core = Path(root) / "src" / "Meltype.Core"
    session = core / "Composition" / "MeltypeSession.cs"
    if session.is_file():
        text = read_text(session)
        if "CompositionServices services" in text:
            return 2
        if "CompositionOptions options" in text:
            return 1
    if (core / "Ime" / "ImeHost.cs").is_file():
        return 3
    return 0


def environment(session, home):
    env = os.environ.copy()
    home = Path(home)
    (home / ".local" / "share").mkdir(parents=True, exist_ok=True)
    env.update(
        {
            "HOME": str(home),
            "XDG_DATA_HOME": str(home / ".local" / "share"),
            "XDG_CONFIG_HOME": str(home / ".config"),
            "MELTYPE_DATA_DIR": str(home / "meltype-data"),
            "DOTNET_CLI_TELEMETRY_OPTOUT": "1",
            "DOTNET_NOLOGO": "1",
            "DOTNET_SKIP_FIRST_TIME_EXPERIENCE": "1",
            "DOTNET_CLI_HOME": str(home),
            "NUGET_PACKAGES": str(home / "nuget"),
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "CARGO_TERM_COLOR": "never",
        }
    )
    return env


# ---------------------------------------------------------------- ハーネス (C#)

CSHARP_VARIANTS = {1: ["gen1"], 2: ["gen2"], 3: ["gen3", "gen3empty"], 0: ["gen3", "gen2", "gen1"]}


def preflight(root, variant):
    """ハーネスが使う API がその版にあるかを、ビルドの前にソースで確かめる。"""
    core = Path(root) / "src" / "Meltype.Core"
    if variant == "gen1":
        text = read_text(core / "Composition" / "MeltypeSession.cs")
        if not re.search(r"CreateDefault\s*\(\s*IKanjiConverter", text):
            raise BenchError("preflight", "MeltypeSession.CreateDefault(IKanjiConverter, …) がありません", "")
        if not re.search(r"public SessionResult HandleKey\(", text):
            raise BenchError("preflight", "MeltypeSession.HandleKey がありません", "")
    elif variant == "gen2":
        text = read_text(core / "Composition" / "MeltypeRuntime.cs")
        if "public MeltypeSession CreateSession(" not in text:
            raise BenchError("preflight", "MeltypeRuntime.CreateSession(IKanjiConverter, …) がありません", "")
        if not (core / "Composition" / "CompositionServices.cs").is_file():
            raise BenchError("preflight", "CompositionServices.cs がありません", "")
    else:
        host = read_text(core / "Ime" / "ImeHost.cs")
        if "public ImeOutput Handle(ImeEvent" not in host:
            raise BenchError("preflight", "ImeHost.Handle(ImeEvent) がありません", "")
        if "public const int Version" not in host:
            raise BenchError("preflight", "ImeHost.Version がありません", "")
        if "SessionParts" not in host:
            raise BenchError("preflight", "SessionParts を受け取る ImeHost ではありません", "")
    return True


def install_csharp_harness(directory, variant, core_project):
    """リポジトリの外に、対象の Meltype.Core を参照する一時プロジェクトを作る。"""
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    template = HARNESS / "csharp" / f"Program.{variant}.cs"
    if not template.is_file():
        raise BenchError("harness", f"ハーネスのテンプレートがありません: {template}")
    project = (HARNESS / "csharp" / "CoreBench.csproj").read_text(encoding="utf-8")
    project = project.replace("__CORE_PROJECT__", str(Path(core_project).resolve()))
    (directory / "CoreBench.csproj").write_text(project, encoding="utf-8")
    shutil.copyfile(template, directory / "Program.cs")
    return directory


def csharp_executable(project_directory):
    candidates = sorted((Path(project_directory) / "bin").glob("Release/*/CoreBench.dll"))
    return candidates[0] if candidates else None


def run_csharp(session, root, cases, data, helper, work, env, variant):
    core_project = Path(root) / "src" / "Meltype.Core" / "Meltype.Core.csproj"
    if not core_project.is_file():
        raise BenchError("build", f"コアのプロジェクトがありません: {core_project}", session.log())
    project_dir = Path(work) / f"harness-{session.target['name']}-{variant}"
    install_csharp_harness(project_dir, variant, core_project)
    session.step(f"dotnet build ({variant})")
    session.run(
        ["dotnet", "build", str(project_dir / "CoreBench.csproj"), "-c", "Release", "--nologo", "-v", "q"],
        cwd=root,
        env=env,
        timeout=1800,
        label=f"dotnet build {variant}",
    )
    cases_path = Path(work) / f"cases-{session.target['name']}.jsonl"
    write_cases(cases_path, cases)
    outputs = {}
    for layer, converter in (("a", None), ("b", helper)):
        if layer == "b" and converter is None:
            continue
        out_path = Path(work) / f"{session.target['name']}-{variant}-{layer}.jsonl"
        if out_path.exists():
            out_path.unlink()
        executable = csharp_executable(project_dir)
        if executable is None:
            raise BenchError("run", "ビルド成果物 CoreBench.dll が見つかりません", session.log())
        command = ["dotnet", str(executable), str(cases_path), str(out_path)]
        if converter is not None:
            command += [str(converter), str(Path(work) / "mozc-profile")]
        session.step(f"measure layer {layer.upper()} ({variant})")
        session.run(command, cwd=root, env=env, timeout=7200, label=f"CoreBench {layer}")
        outputs[layer] = read_jsonl(out_path)
    return outputs


def read_jsonl(path):
    rows = {}
    path = Path(path)
    if not path.is_file():
        return rows
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if "id" in row:
                rows[int(row["id"])] = row
    return rows


# ---------------------------------------------------------------- ハーネス (Rust / 独立実装)

def run_rust_flash(session, root, cases, data, work, env):
    example = Path(root) / "rust" / "core" / "examples" / "core_bench.rs"
    example.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(HARNESS / "rust" / "core_bench.rs", example)
    cases_path = Path(work) / f"cases-{session.target['name']}.jsonl"
    write_cases(cases_path, cases)
    outputs = {}
    plans = [("a", []), ("b", ["--features", "bundled-mozc"])]
    for layer, features in plans:
        if layer == "b" and not session.target.get("layer_b"):
            continue
        session.step(f"cargo build ({layer.upper()}{' +bundled-mozc' if features else ''})")
        base = ["cargo", "build", "--release", "-p", "meltype-core", "--example", "core_bench"] + features
        result = session.run(base + ["--locked"], cwd=root, env=env, timeout=3600, label=f"cargo build {layer}", check=False)
        if result.returncode != 0:
            session.run(base, cwd=root, env=env, timeout=3600, label=f"cargo build {layer} (no lock)")
        out_path = Path(work) / f"{session.target['name']}-{layer}.jsonl"
        if out_path.exists():
            out_path.unlink()
        binary = Path(root) / "target" / "release" / "examples" / "core_bench"
        session.step(f"measure layer {layer.upper()}")
        session.run([str(binary), str(cases_path), str(out_path)], cwd=root, env=env, timeout=7200, label=f"core_bench {layer}")
        outputs[layer] = read_jsonl(out_path)
    return outputs


def cargo_build(session, root, env, label="cargo build"):
    result = session.run(["cargo", "build", "--release", "--locked"], cwd=root, env=env, timeout=3600, label=label, check=False)
    if result.returncode != 0:
        session.run(["cargo", "build", "--release"], cwd=root, env=env, timeout=3600, label=label + " (no lock)")


def run_monakime(session, root, cases, data, helper, work, env):
    session.step("cargo build (monakime-cli)")
    cargo_build(session, root, env)
    binary = Path(root) / "target" / "release" / "monakime-cli"
    if not binary.is_file():
        raise BenchError("build", "monakime-cli がビルドされませんでした", session.log())
    local = env.copy()
    local.update(
        {
            "MONAKIME_DATA_DIR": str(Path(root) / "data"),
            "MONAKIME_ZENZ": "0",
            "MONAKIME_LEARNING": "0",
            "MONAKIME_MOZC_HELPER": str(helper or ""),
            "MONAKIME_MOZC_PROFILE": str(Path(work) / "monakime-mozc"),
        }
    )
    feed = "\n".join(case["typed"] for case in cases) + "\n"
    outputs = {}
    for layer, args in (("a", ["--stdin", "--json", "--no-model"]), ("b", ["--stdin", "--json", "--no-model", "--mozc"])):
        if layer == "b" and (helper is None or not session.target.get("layer_b")):
            continue
        session.step(f"measure layer {layer.upper()} (monakime)")
        result = session.run([str(binary)] + args, cwd=root, env=local, timeout=7200, stdin=feed, label=f"monakime-cli {layer}")
        outputs[layer] = parse_monakime(result.stdout, cases)
    return outputs


def parse_monakime(output, cases):
    """monakime-cli の JSON 行 (1 行 1 変換) を id -> {enter, space} にする。"""
    rows = {}
    for index, line in enumerate((output or "").splitlines()):
        line = line.strip()
        if not line or index >= len(cases):
            continue
        try:
            body = json.loads(line)
        except json.JSONDecodeError:
            continue
        segments = body.get("segments") or []
        text = "".join(str(segment.get("text", "")) for segment in segments)
        if not text:
            candidates = body.get("candidates") or []
            text = str(candidates[0]) if candidates else ""
        rows[cases[index]["id"]] = {"id": cases[index]["id"], "enter": text, "space": text, "ms": body.get("model_ms", 0)}
    return rows


def parse_kswime(output, cases):
    """kswime の標準入力モード (1 行 1 変換、出力も 1 行) を id -> {enter, space} にする。"""
    rows = {}
    lines = (output or "").splitlines()
    for index, case in enumerate(cases):
        text = lines[index].strip() if index < len(lines) else ""
        rows[case["id"]] = {"id": case["id"], "enter": text, "space": text, "ms": 0}
    return rows


def run_kswime(session, root, cases, env):
    session.step("cargo build (kswime)")
    cargo_build(session, root, env)
    binary = Path(root) / "target" / "release" / "kswime"
    if not binary.is_file():
        raise BenchError("build", "kswime がビルドされませんでした", session.log())
    feed = "\n".join(case["typed"] for case in cases) + "\n"
    session.step("measure layer A (kswime)")
    result = session.run([str(binary)], cwd=root, env=env, timeout=7200, stdin=feed, label="kswime")
    return {"a": parse_kswime(result.stdout, cases)}


# ---------------------------------------------------------------- まとめ

def summarize(results, cases):
    """結果の件数と所要時間の分位数。採点は core_aggregate.py が行う。"""
    times = [row["ms"] for row in results if isinstance(row.get("ms"), (int, float))]
    summary = {"cases": len(cases), "measured": len(results)}
    if times:
        ordered = sorted(times)
        summary["p50_ms"] = round(statistics.median(ordered), 3)
        summary["p95_ms"] = round(ordered[min(len(ordered) - 1, int(len(ordered) * 0.95))], 3)
        summary["mean_ms"] = round(sum(times) / len(times), 3)
    return summary


def merge(cases, layers):
    a = layers.get("a") or {}
    b = layers.get("b") or {}
    results = []
    for case in cases:
        first = a.get(case["id"]) or {}
        second = b.get(case["id"]) or {}
        results.append(
            {
                "id": case["id"],
                "a": first.get("enter", ""),
                "b": second.get("enter", ""),
                "bs": second.get("space", ""),
                "ms": second.get("ms", first.get("ms", 0)),
            }
        )
    return results


def measure(session, data, cases, helper, mozc_cache, work_root):
    target = session.target
    kind = target["kind"]
    root, sha = clone(session, Path(work_root) / target["name"])
    env = environment(session, Path(work_root) / "home")
    work = Path(work_root)
    layers = {}
    variant = None
    if kind == "csharp":
        generation = detect_generation(root) or target.get("generation") or 0
        session.step(f"世代 {generation}")
        errors = []
        for candidate in CSHARP_VARIANTS.get(generation, CSHARP_VARIANTS[0]):
            try:
                preflight(root, candidate)
                layers = run_csharp(session, root, cases, data, helper, work, env, candidate)
                variant = candidate
                break
            except BenchError as error:
                errors.append(f"{candidate}: {error.detail}\n{error.log}")
                session.step(f"{candidate} は失敗: {error.detail}")
        if variant is None:
            raise BenchError("build", "C# ハーネスがビルド・実行できませんでした", "\n\n".join(errors))
    elif kind == "rust":
        layers = run_rust_flash(session, root, cases, data, work, env)
    elif kind == "monakime":
        layers = run_monakime(session, root, cases, data, helper, work, env)
    elif kind == "kswime":
        layers = run_kswime(session, root, cases, env)
    else:
        raise BenchError("input", f"未知の種別: {kind}")
    return root, sha, layers, variant


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--target", required=True, help="core_targets.json の対象 ID")
    parser.add_argument("--out", required=True, help="結果 JSON の出力先")
    parser.add_argument("--targets", default=str(TARGETS))
    parser.add_argument("--corpus", default=str(DEFAULT_CORPUS))
    parser.add_argument("--limit", default=os.environ.get("LIMIT"), help="先頭 N 件だけ測る (既定: 環境変数 LIMIT)")
    parser.add_argument("--max-seconds", type=int, default=int(os.environ.get("CORE_BENCH_MAX_SECONDS", DEFAULT_MAX_SECONDS)))
    parser.add_argument("--cache", default=os.environ.get("CORE_BENCH_CACHE") or str(Path(os.environ.get("RUNNER_TEMP", "/tmp")) / "core-bench-cache"))
    parser.add_argument("--work", default=os.environ.get("CORE_BENCH_WORK") or str(Path(os.environ.get("RUNNER_TEMP", "/tmp")) / "core-bench-work"))
    parser.add_argument("--no-layer-b", action="store_true", help="Mozc 変換 (Layer B) を測らない")
    parser.add_argument("--strict", action="store_true", help="失敗したら終了コード 1 にする (既定: 常に 0 で記録だけ残す)")
    args = parser.parse_args()

    started = time.time()
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "version": 1,
        "target": args.target,
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "status": "error",
        "failure": None,
        "steps": [],
        "logs": [],
        "results": [],
        "summary": {},
    }
    session = None
    try:
        data = load_targets(args.targets)
        target = find_target(data, args.target)
        payload.update(
            {
                "name": target["name"],
                "label": target["label"],
                "repo": target["repo"],
                "ref": target["ref"],
                "kind": target["kind"],
                "generation": target.get("generation", 0),
                "layer_b_requested": bool(target.get("layer_b", True)) and not args.no_layer_b,
            }
        )
        cases = load_cases(args.corpus, args.limit)
        payload["cases"] = len(cases)
        payload["limit"] = int(args.limit) if args.limit else None
        Path(args.work).mkdir(parents=True, exist_ok=True)
        session = Session(target, data, args.work, started + args.max_seconds)
        helper = None
        if target.get("layer_b", True) and not args.no_layer_b:
            helper = ensure_mozc(session, args.cache)
        root, sha, layers, variant = measure(session, data, cases, helper, args.cache, args.work)
        payload["sha"] = sha
        payload["harness"] = variant
        payload["results"] = merge(cases, layers)
        payload["summary"] = summarize(payload["results"], cases)
        payload["layers"] = {name: len(rows) for name, rows in layers.items()}
        payload["status"] = "ok" if payload["summary"]["measured"] == len(cases) else "partial"
    except BenchError as error:
        payload["status"] = "error"
        payload["failure"] = {"stage": error.stage, "detail": error.detail, "log": error.log}
    except Exception as error:  # noqa: BLE001 - 何が起きてもジョブを落とさず記録する
        payload["status"] = "error"
        payload["failure"] = {"stage": "unexpected", "detail": f"{type(error).__name__}: {error}", "log": traceback.format_exc()[-4000:]}
    finally:
        if session is not None:
            payload["steps"] = session.steps
            payload["logs"] = session.logs
        payload["seconds"] = round(time.time() - started, 1)
        out_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")
        print(f"[{args.target}] {payload['status']} -> {out_path} ({payload['seconds']}s)", flush=True)
    if args.strict and payload["status"] == "error":
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
