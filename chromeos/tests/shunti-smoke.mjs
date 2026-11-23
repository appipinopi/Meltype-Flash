// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Explicit offline real-model test; model files are never bundled in the extension.
import {readFile} from "node:fs/promises";
import {resolve} from "node:path";
import assert from "node:assert/strict";
import {ShuntiClient,ShuntiPackages} from "../shunti.mjs";
const directory=process.argv[2];
if(!directory)throw Error("model directory required");
const metadata=JSON.parse(await readFile(new URL("../../rust/ai/packages.json",import.meta.url)));
const files={};
for(const name of ["kkc_lex.bin","kkc_model.bin"]){
  const data=await readFile(resolve(directory,name)); files[name]=data.buffer.slice(data.byteOffset,data.byteOffset+data.byteLength);
}
const packages=new ShuntiPackages({metadata,storage:{read:async()=>({revision:"14801bff-s8",files})}});
const wasmBinary=await readFile(new URL("../ai/shunti-engine.wasm",import.meta.url));
const client=new ShuntiClient(packages,{wasmBinary});
await client.prepare(); const module=client.module; await client.prepare(); assert.equal(client.module,module);
const result=client.convert("きょうはいいてんきです","");
assert.equal(result.length,8); assert.ok(result.some(s=>s.includes("今日")));
console.log(JSON.stringify(result)); client.dispose(); assert.deepEqual(client.convert("かな",""),[]);
