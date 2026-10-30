// SPDX-License-Identifier: GPL-3.0-or-later
// Chrome callbacks are registered before asynchronous WASM/storage startup.
import {translateKey} from "./keys.mjs";
const editable = new Set(["text", "search", "url", "email"]);
export class ImeController {
  constructor({ime, load, persist, report, now = () => performance.now(), incognito = false, resolveMode = async () => "normal"}) {
    Object.assign(this, {ime, load, persist, report, now, incognito, resolveMode});
    this.engine = null; this.context = null; this.generation = 0;
    this.queue = Promise.resolve(); this.core = null; this.ownedKeys = new Set();
  }
  ready() {
    if (!this.loading) this.loading = this.load().then(core => (this.core = core));
    return this.loading;
  }
  enqueue(work) {
    const task = this.queue.then(work);
    this.queue = task.catch(error => this.report(String(error)));
    return task;
  }
  invalidate() {
    this.generation++; this.ownedKeys.clear();
    this.refreshMode();
    this.pending = null;
    if (this.core) this.core.reset();
  }
  refreshMode() { this.modeTask = null; }
  async prepareMode(core, engine, id, generation) {
    while (this.valid(engine, id, generation)) {
      const task = this.modeTask ??= {promise: this.resolveMode({...this.context})};
      const mode = await task.promise;
      if (!this.valid(engine, id, generation)) return false;
      // A tab/permission event during lookup must not pass an unconverted letter
      // into a live composition. Resolve the fresh policy for the same key.
      if (this.modeTask !== task) continue;
      // The core defers changes until composition ends; tab events never erase text.
      core.set_code_mode?.(mode === "code");
      return true;
    }
    return false;
  }
  activate(engine) { this.invalidate(); this.engine = engine; this.context = null; }
  deactivate(engine) {
    if (this.engine !== engine) return;
    this.invalidate(); this.engine = null; this.context = null;
  }
  focus(context) {
    this.invalidate();
    this.context = editable.has(context.type) ? {...context} : null;
  }
  updateContext(context) {
    if (this.context?.contextID === context.contextID && this.context.type === context.type
        && this.context.shouldDoLearning === context.shouldDoLearning) {
      this.context = {...context};
    } else this.focus(context);
  }
  blur(id) {
    if (this.context?.contextID !== id) return;
    this.invalidate(); this.context = null;
  }
  valid(engine, id, generation) {
    return this.engine === engine && this.context?.contextID === id && this.generation === generation;
  }
  learning() { return !this.incognito && this.context?.shouldDoLearning !== false; }
  async call(name, parameters) {
    const result = await this.ime[name](parameters);
    if (result === false) throw new Error(`${name} rejected by ChromeOS`);
  }
  async render(result, engine, id, generation) {
    const valid = () => this.valid(engine, id, generation);
    if (result.commits.some(edit => edit.deleteBefore || edit.delete_before || edit.caretBack || edit.caret_back)) {
      throw new Error("This ChromeOS host does not support destructive edits or caret moves");
    }
    // Retain only edits that Chrome has not accepted. A retry must never
    // insert an already accepted prefix twice, even after a partial failure.
    this.pending = {result, remaining: [...result.commits]};
    await this.resume(engine, id, generation);
  }
  async resume(engine, id, generation) {
    const valid = () => this.valid(engine, id, generation);
    const pending = this.pending;
    if (!pending) return;
    const {result, remaining} = pending;
    while (remaining.length) {
      const edit = remaining[0];
      if (!valid()) return;
      try {
        if (edit.text) await this.call("commitText", {contextID: id, text: edit.text});
      } catch (error) {
        // The core has already advanced. Keep the rejected text visible and
        // retry the retained result before allowing the core to advance again.
        if (valid()) {
          const text = remaining.map(edit => edit.text ?? "").join("") + (result.view?.text ?? "");
          if (text) {
            try { await this.call("setComposition", {contextID: id, text, cursor: text.length,
              segments: [{start: 0, end: text.length, style: "underline"}]}); }
            catch (displayError) { this.report(String(displayError)); }
          }
        }
        throw error;
      }
      remaining.shift();
    }
    if (!valid()) return;
    const view = result.view;
    if (view?.text) {
      await this.call("setComposition", {contextID: id, text: view.text, cursor: view.text.length,
        segments: [{start: 0, end: view.text.length, style: "underline"}]});
    } else {
      await this.call("clearComposition", {contextID: id});
    }
    if (!valid()) return;
    const prediction = view && !view.converting && (view.candidates?.length ?? 0) <= 1;
    const candidates = (prediction ? view?.predictions : view?.candidates) ?? [];
    const selected = prediction ? view.selectedPrediction : view?.selectedIndex;
    await this.call("setCandidates", {contextID: id, candidates: candidates.map((candidate, index) => ({
      candidate, id: index, label: index < 9 ? String(index + 1) : "", annotation: prediction ? "" : view.notes?.[index] ?? ""}))});
    if (!valid()) return;
    await this.call("setCandidateWindowProperties", {engineID: engine, properties: {
      visible: candidates.length > 0, vertical: true, cursorVisible: Number.isInteger(selected) && selected >= 0, pageSize: 9,
      auxiliaryText: view?.hint ?? "", auxiliaryTextVisible: Boolean(view?.hint), windowPosition: "composition"}});
    if (valid() && candidates.length && Number.isInteger(selected) && selected >= 0 && selected < candidates.length) await this.call("setCursorPosition", {
      contextID: id, candidateID: selected});
    if (valid() && this.pending === pending) this.pending = null;
  }
  key(engine, event, requestId) {
    // Older ChromeOS sends the acknowledgement ID inside keyData instead of
    // the third listener argument. Missing it used to pass every key through.
    if (typeof requestId !== "string" || !requestId) requestId = event.requestId;
    const mapped = translateKey(event);
    const keyID = event.code || event.key;
    const id = this.context?.contextID, generation = this.generation;
    if ((!mapped && !(event.type === "keyup" && this.ownedKeys.has(keyID)))
        || !this.valid(engine, id, generation) || id === undefined || event.extensionId || event.metaKey) return false;
    if (event.type !== "keydown" && event.type !== "keyup") return false;
    if (typeof requestId !== "string" || !requestId) return false;
    this.enqueue(async () => {
      let handled = false;
      try {
        const core = await this.ready();
        if (!this.valid(engine, id, generation)) return;
        if (event.type === "keyup") {
          handled = this.ownedKeys.delete(keyID);
          return;
        }
        core.focusIfNeeded?.(generation, this.learning());
        if (this.pending) {
          // A failed retry must not let Enter submit the form underneath it.
          if (mapped.vk === 13 || mapped.mode) { handled = true; this.ownedKeys.add(keyID); }
          await this.resume(engine, id, generation);
          if (!this.valid(engine, id, generation)) return;
          if (this.learning()) await this.persist(core.snapshot());
          // 確定・モード切替の再試行で、改行や二度目の切替を起こさない。
          if (mapped.vk === 13 || mapped.mode) {
            return;
          }
        }
        if (!await this.prepareMode(core, engine, id, generation)) return;
        let result;
        if (mapped.mode) {
          handled = true;
          // 押し続けても一度だけ切り替える。keyupは通常の打鍵と同様に引き取る。
          if (this.ownedKeys.has(keyID)) return;
          this.ownedKeys.add(keyID);
          const direct = mapped.mode === "toggle" ? !core.is_direct() : mapped.mode === "direct";
          result = JSON.parse(core.set_direct(direct));
        } else result = JSON.parse(core.key(mapped.vk, mapped.character, mapped.modifiers, this.now()));
        handled = result.consumed;
        if (handled) this.ownedKeys.add(keyID);
        await this.render(result, engine, id, generation);
        if (this.valid(engine, id, generation) && this.learning() && result.commits.length) {
          await this.persist(core.snapshot());
        }
      } catch (error) { this.report(String(error)); }
      finally { this.ime.keyEventHandled(requestId, handled); }
    });
    return undefined;
  }
  candidate(engine, candidateID, button) {
    if (button !== "left" || !Number.isInteger(candidateID) || candidateID < 0) return;
    const id = this.context?.contextID, generation = this.generation;
    if (id === undefined || !this.valid(engine, id, generation)) return;
    this.enqueue(async () => {
      const core = await this.ready();
      if (!this.valid(engine, id, generation)) return;
      if (this.pending) {
        await this.resume(engine, id, generation);
        if (this.valid(engine, id, generation) && this.learning()) await this.persist(core.snapshot());
        return;
      }
      const view = JSON.parse(core.view()).view;
      const prediction = view && !view.converting && (view.candidates?.length ?? 0) <= 1;
      const candidates = (prediction ? view?.predictions : view?.candidates) ?? [];
      if (candidateID >= candidates.length) return;
      const selected = prediction ? core.select_prediction(candidateID) : core.select(candidateID);
      await this.render(JSON.parse(selected), engine, id, generation);
      if (this.valid(engine, id, generation) && this.learning()) await this.persist(core.snapshot());
    });
  }
}
