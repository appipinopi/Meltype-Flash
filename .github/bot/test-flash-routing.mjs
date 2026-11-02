// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// 実botをローカルAPIだけへ接続し、Flashと本家の検証・公開経路を分ける。
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const bot = fileURLToPath(new URL('./meltype-bot.mjs', import.meta.url));
const flash = 'appipinopi/Meltype-Flash';
const upstream = 'yksr-melt/Meltype';
const template = (await fs.readFile(new URL('../ISSUE_TEMPLATE/2-misdetection.yml', import.meta.url), 'utf8')).replace(/\r\n/g, '\n');
// GitHubが本文に使う実フォームの表示名で、botに渡すイベントを構成する。
function formLabel(id) {
  const match = template.match(new RegExp(`^    id: ${id}\\n    attributes:\\n      label: (.+)$`, 'm'));
  assert.ok(match, `フォームの項目が見つかりません: ${id}`);
  return match[1];
}
const form = [['product', 'Meltype-flash（Rust版）'], ['os', 'Windows 11'],
  ['version', '1.0.0'], ['typed', 'nihongowohanasu'], ['actual', 'にほんgo'],
  ['expected', '日本語を話す'], ['key', 'Enter (確定)'], ['kind', '日本語のつもりが英字になった (にほんgo)']]
  .map(([id, value]) => `### ${formLabel(id)}\n\n${value}`).join('\n\n');
const issue = { number: 42, body: form, labels: [{ name: '誤判定' }] };
const commandEvent = body => ({
  action: 'created', issue: { ...issue, pull_request: { url: 'unused' } },
  comment: { id: 7, body, author_association: 'OWNER', user: { type: 'User', login: 'maintainer' } },
});

async function run(repo, event, mode = 'parse', extraEnv = {}) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'meltype-bot-routing-'));
  const requests = [];
  const server = http.createServer(async (request, response) => {
    let raw = '';
    for await (const part of request) raw += part;
    const body = raw ? JSON.parse(raw) : undefined;
    requests.push({ method: request.method, url: request.url, body });
    const value = request.url.includes('/search/issues') ? { items: [] }
      : request.url.includes('/comments') && request.method === 'GET' ? []
        : request.url.endsWith('/permission') ? { permission: 'admin' }
          : request.url.endsWith('/pulls/42') ? { head: { sha: 'a'.repeat(40) } }
          : {};
    response.writeHead(200, { 'Content-Type': 'application/json' });
    response.end(JSON.stringify(value));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  try {
    const eventFile = path.join(directory, 'event.json');
    const outputFile = path.join(directory, 'output.txt');
    await fs.writeFile(eventFile, JSON.stringify(event));
    const env = {
      ...process.env,
      GITHUB_REPOSITORY: repo, GITHUB_EVENT_PATH: eventFile,
      GITHUB_API_URL: `http://127.0.0.1:${server.address().port}`,
      GITHUB_TOKEN: 'test-token', BOT_DISPATCH_TOKEN: 'test-token',
      GITHUB_OUTPUT: outputFile, GITHUB_SERVER_URL: 'https://example.invalid',
      GITHUB_RUN_ID: '123', BOT_ACTION: 'repro', BOT_ISSUE: '42', BOT_JOB_RESULT: 'success',
      ...extraEnv,
    };
    await new Promise((resolve, reject) => {
      const child = spawn(process.execPath, [bot, mode], { env, stdio: ['ignore', 'pipe', 'pipe'] });
      let stderr = '';
      child.stderr.on('data', part => { stderr += part; });
      child.on('error', reject);
      child.on('close', code => code === 0 ? resolve() : reject(new Error(`bot exit ${code}: ${stderr}`)));
    });
    const rawOutput = await fs.readFile(outputFile, 'utf8').catch(error => {
      if (error.code === 'ENOENT') return '';
      throw error;
    });
    const output = {};
    for (const match of rawOutput.matchAll(/(\w+)<<(EOF_\w+)\n([\s\S]*?)\n\2\n/g)) output[match[1]] = match[3];
    return { requests, output };
  } finally {
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
    await fs.rm(directory, { recursive: true, force: true });
  }
}

const comments = result => result.requests.filter(r => r.method === 'POST' && /\/issues\/42\/comments$/.test(r.url)).map(r => r.body.body);
const dispatches = result => result.requests.filter(r => /\/git\/refs|\/releases|\/dispatches|\/contents\//.test(r.url));

test('Flashの誤判定報告はRust検証へ案内しC#再現を起動しない', async () => {
  const result = await run(flash, { action: 'opened', issue });
  assert.equal(result.output.action, 'none');
  assert.equal(result.output.release_command, undefined);
  assert.ok(comments(result).some(body => body.includes('Rust版') && body.includes('rust.yml')));
  assert.ok(!result.requests.some(r => r.body?.labels?.includes('再現済み')));
  assert.deepEqual(dispatches(result), []);
});

test('本家の誤判定報告は既存のC#再現へ渡す', async () => {
  const result = await run(upstream, { action: 'opened', issue });
  assert.equal(result.output.action, 'repro');
  assert.equal(result.output.keys, 'nihongowohanasu');
  assert.equal(result.output.reported, 'にほんgo');
  assert.equal(result.output.expected, '日本語を話す');
  assert.equal(result.output.kind, '日本語のつもりが英字になった (にほんgo)');
  assert.equal(result.output.last, 'enter');
  assert.ok(!comments(result).some(body => body.includes('rust-release.yml')));
});

for (const command of ['repro nihongowohanasu', 'explain hello', 'ca kisha', 'jht 日本語',
  `test ${'a'.repeat(40)}`, `pack ${'a'.repeat(40)}`, `test-repro ${'a'.repeat(40)} hello`,
  `test-jht ${'a'.repeat(40)} 日本語`, `test-explain ${'a'.repeat(40)} hello`, 'release 1.0.0',
  'pre-release 1.0.0-pre.1', 'help']) {
  test(`Flashの/${command.split(' ')[0]}はC#処理をdispatchしない`, async () => {
    const result = await run(flash, commandEvent('/' + command));
    assert.equal(result.output.action, 'none');
    assert.equal(result.output.release_command, undefined);
    assert.deepEqual(dispatches(result), []);
    assert.ok(comments(result).some(body => body.includes('rust.yml') && body.includes('rust-release.yml')));
  });
}

test('Flashでは古いC#結果から再現済みlabelや成功reactionを付けない', async () => {
  const result = await run(flash, commandEvent('/repro hello'), 'report');
  assert.ok(!result.requests.some(r => r.body?.labels?.includes('再現済み')));
  assert.ok(!result.requests.some(r => r.url.endsWith('/reactions')));
  assert.ok(comments(result).some(body => body.includes('Flashの「再現済み」や完了の判定には使いません')));
});

test('Flashはrelease jobを直接呼ばれても本家タグ・配布物を作らない', async () => {
  const result = await run(flash, commandEvent('/release 1.0.0'), 'release', {
    BOT_RELEASE_COMMAND: 'release', BOT_RELEASE_VERSION: '1.0.0',
  });
  assert.deepEqual(dispatches(result), []);
  assert.ok(comments(result).some(body => body.includes('rust-release.yml')));
});

test('本家のrelease parseは既存の権限付きjobへ渡す', async () => {
  const result = await run(upstream, commandEvent('/release 1.4.0'));
  assert.equal(result.output.release_command, 'release');
  assert.equal(result.output.release_version, '1.4.0');
  assert.deepEqual(dispatches(result), []);
});

test('本家の確認済みpack命令は既存PR jobへ渡す', async () => {
  const result = await run(upstream, commandEvent('/pack ' + 'a'.repeat(40)));
  assert.equal(result.output.action, 'pack');
  assert.equal(result.output.sha, 'a'.repeat(40));
});
