// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import {test} from "node:test";
import assert from "node:assert/strict";
import {createHash} from "node:crypto";
import {ShuntiPackages, ShuntiProxy} from "../shunti.mjs";

function fixture({corrupt = false, fail = false} = {}) {
  let value, writes = 0, requests = 0;
  const bytes = [new Uint8Array([1,2,3]), new Uint8Array([4,5,6])];
  const metadata = {shunti:{revision:"14801bff-s8",files:bytes.map((data,index)=>({
    name:index ? "kkc_model.bin" : "kkc_lex.bin",
    url:`https://github.com/shuntilettuce/Japanese-IME-for-HarmonyOS-next/releases/download/shuntelligence-v7/${index}`,
    sha256:createHash("sha256").update(data).digest("hex")
  }))}};
  const packages = new ShuntiPackages({metadata,storage:{read:async()=>value,write:async data=>{value=data;writes++;}},
    fetcher:async()=>{const n=requests++; if(fail&&n===1)throw Error("interrupted"); return new Response(corrupt?new Uint8Array([0]):bytes[n]);}});
  return {packages, metadata, writes:()=>writes, requests:()=>requests, value:()=>value};
}
test("downloads publish one complete verified package and reuse it offline", async()=>{
  const f=fixture(); assert.equal(await f.packages.installed(),false);
  await f.packages.install(); assert.equal(f.requests(),2); assert.equal(f.writes(),1);
  assert.equal(await f.packages.installed(),true);
  await f.packages.install(); assert.equal(f.requests(),2);
  new Uint8Array(f.value().files["kkc_lex.bin"])[0]=0;
  await assert.rejects(f.packages.load(),/変更/);
});
test("interrupted or altered downloads leave no partially installed environment", async()=>{
  for(const mode of [{corrupt:true},{fail:true}]){
    const f=fixture(mode); await assert.rejects(f.packages.install());
    assert.equal(f.writes(),0); assert.equal(await f.packages.installed(),false);
  }
  const f=fixture(); f.metadata.shunti.files[0].url="https://example.com/model";
  await assert.rejects(f.packages.install(),/不正/); assert.equal(f.requests(),0);
});
test("slow AI releases the IME event after its deadline and does not queue more inference",async t=>{
  const previous=globalThis.chrome; t.after(()=>{globalThis.chrome=previous;});
  let complete,requests=0;
  globalThis.chrome={runtime:{sendMessage:()=>{requests++;return new Promise(resolve=>{complete=resolve;});}}};
  const proxy=new ShuntiProxy(); proxy.ready=true;
  const start=performance.now(); assert.deepEqual(await proxy.convert("にほんご",""),[]);
  assert.ok(performance.now()-start<1500); assert.equal(requests,1);
  assert.deepEqual(await proxy.convert("にほんご",""),[]); assert.equal(requests,1);
  complete({candidates:["日本語"]}); await new Promise(resolve=>setImmediate(resolve));
  assert.equal(proxy.inFlight,false);
});
