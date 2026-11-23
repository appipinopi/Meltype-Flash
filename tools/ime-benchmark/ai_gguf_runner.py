#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 appipinopi
"""GGUFモデル1種×1シャードを llama-server で測る。

本番 (`rust/native/src/windows_converter/zenzai.rs`) と同じ起動引数・同じプロンプト・
同じ応答の後処理で、1万文コーパスを id 順に変換し、1文ごとの出力と実時間を
JSONL に追記する (期限内に終わらなければ途中までの結果を残して止まる)。

  python tools/ime-benchmark/ai_gguf_runner.py \
      --model zenz-v1-q8_0 --shard 0 --shards 1 --out bench-out/zenz-v1-q8_0.jsonl

1文の出力は本番と同じ「文脈なし (context = '。')」で作る。コーパスの読みは
すべて3文字以上なので、本番の2文字ガードと80文字ガードには当たらない。
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ai_protocol import clean, prompt  # noqa: E402

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
CATALOG = ROOT / "rust" / "ai" / "model-catalog.json"
SETUP = ROOT / "rust" / "setup-ai-linux.py"
DEFAULT_CORPUS = HERE / "data"
CONTEXT = "。"
OPENER = urllib.request.build_opener(urllib.request.ProxyHandler({}))


class Server:
    """127.0.0.1 の llama-server。GET /health が 200 になってから使う。"""

    def __init__(self, process, port):
        self.process = process
        self.port = port
        self.base = f"http://127.0.0.1:{port}"

    def alive(self):
        return self.process is None or self.process.poll() is None

    def wait_healthy(self, limit=600.0):
        deadline = time.monotonic() + limit
        while time.monotonic() < deadline:
            if not self.alive():
                raise RuntimeError(
                    f"llama-server が起動中に終了しました (exit={self.process.returncode})"
                )
            try:
                with OPENER.open(self.base + "/health", timeout=3) as response:
                    if response.status == 200:
                        return
            except urllib.error.HTTPError:
                pass
            except OSError:
                pass
            time.sleep(0.25)
        raise RuntimeError("llama-server の起動が時間内に終わりませんでした")

    def completion(self, text, timeout):
        body = json.dumps(
            {
                "prompt": text,
                "n_predict": 96,
                "temperature": 0,
                "top_k": 1,
                "cache_prompt": True,
                "parse_special": True,
            }
        ).encode()
        request = urllib.request.Request(
            self.base + "/completion",
            data=body,
            headers={"Content-Type": "application/json"},
        )
        with OPENER.open(request, timeout=timeout) as response:
            payload = json.loads(response.read())
        content = payload.get("content")
        if not isinstance(content, str):
            raise RuntimeError("completion に content がありません")
        return content

    def stop(self):
        if self.process is None or self.process.poll() is not None:
            return
        self.process.terminate()
        try:
            self.process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=15)


def load_gguf_models():
    models = json.loads(CATALOG.read_text(encoding="utf-8"))["models"]
    return {m["id"]: m for m in models}


def load_cases(corpus):
    path = Path(corpus)
    paths = sorted(path.glob("*.jsonl")) if path.is_dir() else [path]
    rows = []
    for part in paths:
        with part.open(encoding="utf-8") as source:
            rows.extend(json.loads(line) for line in source if line.strip())
    rows.sort(key=lambda row: row["id"])
    return [{"id": row["id"], "input": row["input"]} for row in rows]


def shard_slice(cases, shard, shards):
    """id 順の一覧を shards 等分し、shard 番目を返す (空にもなりうる)。"""
    total = len(cases)
    return cases[total * shard // shards : total * (shard + 1) // shards]


def ensure_install(model, data_root):
    """`rust/setup-ai-linux.py` で llama-server と検証済みモデルを導入する。"""
    data_root = Path(data_root).resolve()
    data_root.mkdir(parents=True, exist_ok=True)
    spec = data_root / f"spec-{model['id']}.json"
    spec.write_text(json.dumps(model, ensure_ascii=False), encoding="utf-8")
    result = subprocess.run(
        [sys.executable, str(SETUP), str(data_root), str(spec)],
        check=True,
        capture_output=True,
        text=True,
    )
    paths = json.loads(result.stdout.strip().splitlines()[-1])
    return Path(paths["server"]), Path(paths["model"])


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def launch(server_path, model_path, zenz_tokenizer, log_path):
    """zenzai.rs と同じ引数で llama-server を起動する。"""
    port = free_port()
    command = [
        str(server_path),
        "-m",
        str(model_path),
        "--host",
        "127.0.0.1",
        "--port",
        str(port),
        "-c",
        "1024",
        "-ngl",
        "0",
        "--parallel",
        "1",
    ]
    if zenz_tokenizer:
        command += ["--override-kv", "tokenizer.ggml.pre=str:gpt-2"]
    log = open(log_path, "wb")
    try:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
    finally:
        log.close()
    return Server(process, port)


def percentile(values, ratio):
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, int(round(ratio * (len(ordered) - 1)))))
    return ordered[index]


def run(args):
    models = load_gguf_models()
    model = models.get(args.model)
    if model is None:
        raise SystemExit(f"model-catalog.json に {args.model} がありません")
    if args.shards < 1 or not 0 <= args.shard < args.shards:
        raise SystemExit("--shard は 0 以上 --shards 未満で指定してください")

    cases = load_cases(args.corpus)
    if args.limit:
        cases = cases[: args.limit]
    shard_cases = shard_slice(cases, args.shard, args.shards)
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    meta_path = Path(args.meta) if args.meta else out_path.with_suffix(".meta.json")
    started_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    began = time.monotonic()

    meta = {
        "model": model["id"],
        "family": model["family"],
        "protocol": model["protocol"],
        "shard": args.shard,
        "shards": args.shards,
        "limit": args.limit,
        "corpus_cases": len(cases),
        "cases_in_shard": len(shard_cases),
        "completed": 0,
        "reason": "not_started",
        "started_at": started_at,
        "context": CONTEXT,
        "zenz_tokenizer": model["family"] == "Zenz",
        "runner": "llama-server",
    }

    latencies = []
    errors = 0
    server = None
    with out_path.open("w", encoding="utf-8") as out:
        try:
            install_began = time.monotonic()
            server_path, model_path = ensure_install(model, args.data_root)
            meta["install_seconds"] = round(time.monotonic() - install_began, 1)
            meta["server"] = str(server_path)
            meta["model_file"] = str(model_path)
            startup_began = time.monotonic()
            server = launch(
                server_path,
                model_path,
                model["family"] == "Zenz",
                out_path.with_suffix(".server.log"),
            )
            server.wait_healthy(args.startup_timeout)
            meta["startup_seconds"] = round(time.monotonic() - startup_began, 1)
            meta["reason"] = "running"
            for position, case in enumerate(shard_cases):
                if args.max_seconds and time.monotonic() - began > args.max_seconds:
                    meta["reason"] = "deadline"
                    break
                if not server.alive():
                    meta["reason"] = "server_exit"
                    break
                text = prompt(model["protocol"], case["input"], CONTEXT)
                began_case = time.perf_counter()
                error = None
                try:
                    actual = clean(server.completion(text, args.request_timeout)) or ""
                except Exception as failure:  # 1文の失敗で全体を止めない
                    actual = ""
                    error = str(failure)[:300]
                    errors += 1
                ms = (time.perf_counter() - began_case) * 1000
                latencies.append(ms)
                row = {"id": case["id"], "actual": actual, "ms": round(ms, 3)}
                if error:
                    row["error"] = error
                out.write(json.dumps(row, ensure_ascii=False) + "\n")
                out.flush()
                meta["completed"] = position + 1
                if error and errors >= 5:
                    meta["reason"] = "request_errors"
                    break
            else:
                meta["reason"] = "completed"
        except Exception as failure:
            meta["reason"] = meta["reason"] if meta["reason"] == "deadline" else "failed"
            meta["error"] = str(failure)[:1000]
            raise
        finally:
            if server is not None:
                server.stop()
            meta["elapsed_seconds"] = round(time.monotonic() - began, 1)
            meta["errors"] = errors
            meta["p50_ms"] = round(percentile(latencies, 0.50), 3)
            meta["p95_ms"] = round(percentile(latencies, 0.95), 3)
            meta["mean_ms"] = round(sum(latencies) / len(latencies), 3) if latencies else 0.0
            meta["finished_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            meta_path.write_text(
                json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
            )
    summary = (
        f"{model['id']} shard {args.shard}/{args.shards}: "
        f"{meta['completed']}/{meta['cases_in_shard']} ({meta['reason']}) "
        f"p50 {meta['p50_ms']}ms / mean {meta['mean_ms']}ms"
    )
    print(summary)
    step_summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if step_summary:
        with open(step_summary, "a", encoding="utf-8") as target:
            target.write(f"- {summary}\n")
    return meta


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", required=True, help="model-catalog.json の id")
    parser.add_argument("--shard", type=int, default=0)
    parser.add_argument("--shards", type=int, default=1)
    parser.add_argument("--corpus", default=str(DEFAULT_CORPUS))
    parser.add_argument("--out", required=True, help="1文ごとの結果 (JSONL)")
    parser.add_argument("--meta", default=None, help="実行メタ情報 (JSON)")
    parser.add_argument("--data-root", default=str(HERE / ".ai-bench"))
    parser.add_argument(
        "--limit",
        type=int,
        default=int(os.environ.get("AI_BENCH_LIMIT") or 0),
        help="先頭N件だけ測る (0=全件)",
    )
    parser.add_argument(
        "--max-seconds",
        type=float,
        default=float(os.environ.get("AI_BENCH_MAX_SECONDS") or 0),
        help="この秒数を超えたら途中で止める (0=制限なし)",
    )
    parser.add_argument("--request-timeout", type=float, default=300.0)
    parser.add_argument("--startup-timeout", type=float, default=600.0)
    args = parser.parse_args()
    run(args)


if __name__ == "__main__":
    main()
