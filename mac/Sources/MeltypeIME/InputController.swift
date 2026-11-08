// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Yukishiro

import Carbon.HIToolbox
import Cocoa
import InputMethodKit

/// 入力欄 (クライアント) ごとの IME。キーを本体 (libMeltypeNative.dylib) に渡し、
/// 返ってきた結果 (確定する文字・変換中の表示・候補) を入力欄に反映する。
/// Info.plist の InputMethodServerControllerClass に書いた名前で、Input Method Kit が作る。
@objc(MeltypeInputController)
final class MeltypeInputController: IMKInputController, ModeServiceTarget {
    private var session: UnsafeMutableRawPointer?
    private var candidateList: [String] = []
    private var hasMarkedText = false
    private var codeInput = ModeSelection.shared.current == .code
    private var directInput = ModeSelection.shared.current == .direct
    private var mode: MacInputMode { directInput ? .direct : codeInput ? .code : .automatic }
    private let modeIndicator = ModeIndicator.shared
    private var suggestionPanel: NSPanel?
    private var displayedSuggestion: String?
    private var dictionaryObserver: NSObjectProtocol?

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        ModeServices.shared.activate(self)
        let selected = ModeSelection.shared.current
        if mode != selected {
            directInput = selected == .direct
            codeInput = selected == .code
        }
#if MELTYPE_RUST_CORE
        let application = NSWorkspace.shared.frontmostApplication
        let identity = application?.bundleIdentifier ?? application?.executableURL?.lastPathComponent ?? ""
        let applicationCode = NativeCore.shared.setApplication(session, identity)
        if selected == .automatic { codeInput = applicationCode }
#endif
        NativeCore.shared.setDirect(session, directInput)
        NativeCore.shared.setCodeInput(session, codeInput)
    }

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        session = NativeCore.shared.createSession()
        NativeCore.shared.setDirect(session, directInput)
        NativeCore.shared.setCodeInput(session, codeInput)
        // 送るのは同じプロセスの importUserDictionary だけなので、ほかのプロセスから届く分散通知では受けない
        dictionaryObserver = NotificationCenter.default.addObserver(
            forName: Notification.Name("MeltypeUserDictionaryChanged"), object: nil, queue: .main
        ) { [weak self] _ in
            guard let self else { return }
            self.commitComposition(self.client())
            NativeCore.shared.destroySession(self.session)
            self.session = NativeCore.shared.createSession()
            NativeCore.shared.setDirect(self.session, self.directInput)
            NativeCore.shared.setCodeInput(self.session, self.codeInput)
        }
    }

    deinit {
        if let dictionaryObserver { NotificationCenter.default.removeObserver(dictionaryObserver) }
        NativeCore.shared.destroySession(session)
    }

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask.keyDown.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else { return false }
        if event.keyCode == kVK_Space, event.characters == "　" || event.characters == "\u{00A0}",
           event.modifierFlags.intersection([.command, .control]).isEmpty {
            apply(NativeCore.shared.commit(session), to: client)
            client.insertText(NSAttributedString(string: " "), replacementRange: NSRange(location: NSNotFound, length: 0))
            return true
        }

        // JIS キーボードの「英数」「かな」キー: 英数 (直接入力) ⇔ 日本語。
        switch Int(event.keyCode) {
        case kVK_JIS_Eisu:
            apply(NativeCore.shared.commit(session), to: client)
            directInput = true
            NativeCore.shared.setDirect(session, true)
            rememberMode(caret: modeIndicatorCaret(client))
            return true
        case kVK_JIS_Kana:
            directInput = false
            codeInput = false
            NativeCore.shared.setCodeInput(session, false)
            NativeCore.shared.setDirect(session, false)
            rememberMode(caret: modeIndicatorCaret(client))
            return true
        default:
            break
        }

        guard let vk = KeyMapping.virtualKey(for: event) else { return false }
        // event.characters は「見た目の 1 文字」で、UTF-16 要素が複数のことがある
        // (補助面の文字・結合文字・ZWJ 絵文字・異体字セレクター)。1 要素へ切り詰めたり 0 に置き換えたりしない。
        // 1 スカラーならそのコードポイントのまま本体へ渡し、複数スカラーは未確定内容だけ確定して
        // 元のイベントを 1 回アプリへ通す (本体側の pass-through 契約と対にする)。
        let scalars = Array((event.characters ?? "").unicodeScalars)
        if scalars.count > 1 {
            apply(NativeCore.shared.commitBeforeExternalText(session, text: event.characters ?? ""), to: client)
            return false
        }
        let character: Int32 = scalars.count == 1 ? Int32(scalars[0].value) : 0
        let modifiers = KeyMapping.modifiers(for: event.modifierFlags)

        let (before, after) = hasMarkedText ? (nil, nil) : surroundingText(of: client)
        guard let result = NativeCore.shared.handleKey(session, vk: vk, character: character, modifiers: modifiers, before: before, after: after) else {
            return false
        }
        apply(result, to: client)
        return result.consumed
    }

    /// フォーカスが外れた・クリックで別の場所に移ったときなど。未確定の内容をそのまま確定する。
    override func commitComposition(_ sender: Any!) {
        guard let client = (sender as? IMKTextInput) ?? self.client() else { return }
        apply(NativeCore.shared.commit(session), to: client)
    }

    override func deactivateServer(_ sender: Any!) {
        ModeServices.shared.deactivate(self)
        commitComposition(sender)
        hideCandidatesWindow()
        hideSuggestion()
        clearMeaning()
        candidateList = []
        super.deactivateServer(sender)
    }

    // ---- 変換の候補の一覧 ----

    override func candidates(_ sender: Any!) -> [Any]! {
        candidateList
    }

    override func candidateSelected(_ candidateString: NSAttributedString!) {
        guard let client = self.client(),
              let string = candidateString?.string,
              let index = candidateList.firstIndex(of: string) else { return }
        // クリックした候補が候補ウィンドウで選ばれているので、覚えている位置も合わせる。
        windowIndex = index
        apply(NativeCore.shared.selectCandidate(session, index: index), to: client)
    }

    /// 候補ウィンドウの中で選択が動いた (マウスで選んだときなど)。本体の選択も合わせる。
    /// 本体の選択を候補ウィンドウに反映しているとき (selectingFromCore) にも呼ばれるので、そのときは何もしない。
    /// moveDown / moveUp の通知が遅れて届くと古い番号に戻してしまうので、マウスの操作のときだけ受け付ける。
    private var selectingFromCore = false

    override func candidateSelectionChanged(_ candidateString: NSAttributedString!) {
        let mouseTypes: [NSEvent.EventType] = [.leftMouseDown, .leftMouseUp, .leftMouseDragged]
        guard !selectingFromCore,
              let type = NSApp.currentEvent?.type, mouseTypes.contains(type),
              let client = self.client(),
              let string = candidateString?.string,
              let index = candidateList.firstIndex(of: string) else { return }
        windowIndex = index
        apply(NativeCore.shared.selectCandidate(session, index: index), to: client)
    }

    // ---- メニュー (メニューバーの入力メニュー) ----

    override func menu() -> NSMenu! {
#if MELTYPE_RUST_CORE
        return NativeCore.shared.editorPolicy?.inputMenu(target:self,direct:directInput,code:codeInput,indicator:ModeIndicatorSettings.shared.isEnabled,analytics:FlashAnalytics.isEnabled) as? NSMenu
#else
        let menu = NSMenu()
        for (title, action, selected) in [
            ("日本語・英語の自動判定", #selector(selectAutomatic(_:)), !codeInput && !directInput),
            ("英数の直接入力", #selector(selectDirect(_:)), directInput),
            ("コード入力 (コメント・文字列は日本語)", #selector(selectCode(_:)), codeInput && !directInput)
        ] {
            let item = menu.addItem(withTitle: title, action: action, keyEquivalent: "")
            item.target = self
            item.state = selected ? .on : .off
        }
        menu.addItem(.separator())
        let indicator = menu.addItem(withTitle: "入力モードをミニウィンドウで表示", action: #selector(toggleModeIndicator(_:)), keyEquivalent: "")
        indicator.target = self
        indicator.state = ModeIndicatorSettings.shared.isEnabled ? .on : .off
        let importItem = menu.addItem(withTitle: "macOSのユーザー辞書を取り込む…", action: #selector(importUserDictionary(_:)), keyEquivalent: "")
        importItem.target = self
        let configItem = menu.addItem(withTitle: "Meltype の設定ファイル (config.json) を開く…", action: #selector(openConfigFile(_:)), keyEquivalent: "")
        configItem.target = self
        menu.addItem(withTitle: "Meltype のデータフォルダを開く (設定・ユーザー辞書)", action: #selector(openDataFolder(_:)), keyEquivalent: "")
        menu.addItem(.separator())
        let analytics = menu.addItem(withTitle: "利用状況を匿名で送る", action: #selector(toggleAnalytics(_:)), keyEquivalent: "")
        analytics.target = self
        analytics.state = FlashAnalytics.isEnabled ? .on : .off
        menu.addItem(withTitle: "不具合の報告・提案… (Mac 版はプレビュー版です)", action: #selector(openReport(_:)), keyEquivalent: "")
        return menu
#endif
    }

    private func selectMode(direct: Bool, code: Bool, showIndicator: Bool = false) {
        let inputClient = client()
        let caret = inputClient.flatMap { modeIndicatorCaret($0) }
        commitComposition(inputClient)
        directInput = direct
        codeInput = code
        NativeCore.shared.setDirect(session, direct)
        NativeCore.shared.setCodeInput(session, code)
        rememberMode(caret: inputClient.flatMap { modeIndicatorCaret($0) } ?? caret, showIndicator: showIndicator)
    }

    /// メニューと同じく、現在の表示を確定してからモードとミニウィンドウを更新する。
    func selectModeFromService(_ action: ModeShortcutAction) -> Bool {
        guard client() != nil else { return false }
        let selected = action.mode(after: mode)
        selectMode(direct: selected == .direct, code: selected == .code, showIndicator: true)
        return true
    }

    private func rememberMode(caret: NSRect?, showIndicator: Bool = true) {
        ModeSelection.shared.select(mode)
        if showIndicator {
            modeIndicator.show(mode, caret: caret)
        } else {
            modeIndicator.hide()
        }
    }

    private func modeIndicatorCaret(_ client: IMKTextInput) -> NSRect? {
        // IMK のインライン位置。0 はインラインセッションがない場合の現在の選択を表す。
        let marked = client.markedRange()
        let index = marked.location == NSNotFound ? 0 : marked.length
        var rectangle = NSRect.zero
        _ = client.attributes(forCharacterIndex: index, lineHeightRectangle: &rectangle)
        return ModeIndicator.validCaret(rectangle) ? rectangle : nil
    }

    @objc private func selectAutomatic(_ sender: Any?) { selectMode(direct: false, code: false) }
    @objc private func selectDirect(_ sender: Any?) { selectMode(direct: true, code: false) }
    @objc private func selectCode(_ sender: Any?) { selectMode(direct: false, code: true) }

    @objc private func toggleModeIndicator(_ sender: Any?) {
        ModeIndicatorSettings.shared.isEnabled.toggle()
        modeIndicator.hide()
    }

    /// 利用状況の集計は、入力を預かるアプリなので既定は無効。入れた人だけ送る。
    @objc private func toggleAnalytics(_ sender: Any?) {
        FlashAnalytics.setEnabled(!FlashAnalytics.isEnabled)
    }

    @objc private func importUserDictionary(_ sender: Any?) {
        commitComposition(client())
#if MELTYPE_RUST_CORE
        NativeCore.shared.editorPolicy?.showDictionaryImport { NativeCore.shared.dataDirectory }
#else
        // 入力メソッドは背景のアプリなので、前面に出さないと、開いた画面がほかのウィンドウに触れた瞬間に消える (issue #716)
        NSApp.activate(ignoringOtherApps: true)
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.level = .floating
        panel.makeKeyAndOrderFront(nil)
        guard panel.runModal() == .OK, let source = panel.url,
              let directory = NativeCore.shared.dataDirectory else { return }
        NSApp.activate(ignoringOtherApps: true)
        let alert = NSAlert()
        do {
            let count = try MacUserDictionary.importFile(source, directory: directory)
            alert.messageText = "ユーザー辞書に\(count)語を追加しました"
            NotificationCenter.default.post(name: Notification.Name("MeltypeUserDictionaryChanged"), object: nil)
        } catch {
            alert.messageText = "ユーザー辞書を取り込めませんでした"
            alert.informativeText = error.localizedDescription
        }
        alert.runModal()
#endif
    }

    @objc private func openReport(_ sender: Any?) {
        guard let url = NativeCore.shared.reportUrl else { return }
        NSWorkspace.shared.open(url)
    }

    /// 設定ファイル (Windows 版と共通の config.json) を、無ければ空の設定で作ってから、既定のエディターで開く (issue #508)。
    @objc private func openConfigFile(_ sender: Any?) {
        guard let directory = NativeCore.shared.dataDirectory else { return }
        let url = URL(fileURLWithPath: directory, isDirectory: true).appendingPathComponent("config.json")
        if !FileManager.default.fileExists(atPath: url.path) {
            try? "{\n}\n".write(to: url, atomically: true, encoding: .utf8)
        }
        NSWorkspace.shared.open(url)
    }

    @objc private func openDataFolder(_ sender: Any?) {
        guard let directory = NativeCore.shared.dataDirectory else { return }
        NSWorkspace.shared.open(URL(fileURLWithPath: directory, isDirectory: true))
    }

    // ---- 結果を入力欄に反映する ----

    func apply(_ result: SessionResult?, to client: IMKTextInput) {
        guard let result else { return }
#if MELTYPE_RUST_CORE
        NativeCore.shared.editorPolicy?.apply(result,
            marked: { client.markedRange() }, selected: { client.selectedRange() },
            substring: { client.attributedSubstring(from: $0)?.string },
            insert: { text, range in
                client.insertText(NSAttributedString(string: text), replacementRange: range)
                self.hasMarkedText = false
            }, clearMarked: {
                client.setMarkedText(NSAttributedString(string: ""), selectionRange: NSRange(location: 0, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
                self.hasMarkedText = false
            }, render: {
                if let view = result.view { self.showComposition(view, client: client) }
                else { self.hideComposition(client: client) }
            })
#else
        for edit in result.commits {
            var range = NSRange(location: NSNotFound, length: 0)
            if edit.deleteBefore > 0 {
                if let verified = verifiedRange(for: edit, client: client) {
                    // 確定し直し: キャレットの前の文字を置き換える。
                    range = verified
                } else {
                    // 消す文字を確認できないときは、前の文字は消さずに今打った文字だけを入れる
                    // (本体は text が入った前提で状態を進めるので、edit ごと捨てない)。
                    NSLog("Meltype: 確定し直す文字を確認できないため、前の文字を消しません")
                }
            }
            client.insertText(NSAttributedString(string: edit.text), replacementRange: range)
            hasMarkedText = false
        }
        if let view = result.view {
            showComposition(view, client: client)
        } else {
            hideComposition(client: client)
        }
#endif
    }

    /// 消してよいと確認できた範囲 (キャレット (変換中の文字があればその先頭) の前の DeleteBefore 文字)。確認できないときは nil。
    private func verifiedRange(for edit: TextEdit, client: IMKTextInput) -> NSRange? {
        let marked = client.markedRange()
        let caret = marked.location != NSNotFound && marked.length > 0 ? marked.location : client.selectedRange().location
        guard caret != NSNotFound, caret >= edit.deleteBefore else { return nil }
        let candidate = NSRange(location: caret - edit.deleteBefore, length: edit.deleteBefore)
        let previous = client.attributedSubstring(from: candidate)?.string
        guard matchesExpectedText(previous, expected: edit.expect, deleteBefore: edit.deleteBefore) else { return nil }
        return candidate
    }

    /// 変換中の文字を入力欄に下線付きで出す (変換中は文節ごと、選んでいる文節は太い下線)。
    private func showComposition(_ view: CompositionView, client: IMKTextInput) {
        let text = NSMutableAttributedString(string: view.text)
#if MELTYPE_RUST_CORE
        guard NativeCore.shared.editorPolicy?.composition(view, mark: { style, range in
            let carbonStyle = [kTSMHiliteRawText, kTSMHiliteConvertedText, kTSMHiliteSelectedConvertedText][style]
            self.addMark(carbonStyle, to: text, range: range)
        }, publish: { length in
            client.setMarkedText(NSAttributedString(attributedString: text), selectionRange: NSRange(location: length, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
            self.hasMarkedText = length > 0
        }) == true else { return }
#else
        let length = (view.text as NSString).length
        if view.converting && !view.clauses.isEmpty {
            var location = 0
            for (index, clause) in view.clauses.enumerated() {
                let clauseLength = (clause as NSString).length
                let style = index == view.selectedClause ? kTSMHiliteSelectedConvertedText : kTSMHiliteConvertedText
                addMark(style, to: text, range: NSRange(location: location, length: clauseLength))
                location += clauseLength
            }
        } else {
            addMark(kTSMHiliteRawText, to: text, range: NSRange(location: 0, length: length))
        }
        client.setMarkedText(NSAttributedString(attributedString: text), selectionRange: NSRange(location: length, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
        hasMarkedText = length > 0
#endif
        updateCandidates(view)
        showSuggestion(view.suggestion, client: client)
    }

    private func hideComposition(client: IMKTextInput) {
        if hasMarkedText {
            // Keep the payload type consistent when clearing marked text.
            client.setMarkedText(NSAttributedString(string: ""), selectionRange: NSRange(location: 0, length: 0), replacementRange: NSRange(location: NSNotFound, length: 0))
            hasMarkedText = false
        }
        candidateList = []
        hideCandidatesWindow()
        hideSuggestion()
        clearMeaning()
    }

    private func hideSuggestion() {
#if MELTYPE_RUST_CORE
        if let panel=NativeCore.shared.editorPolicy?.suggestion(panel:suggestionPanel,text:nil,caret:.zero,selected:false) as? NSPanel { suggestionPanel=panel }
#else
        suggestionPanel?.orderOut(nil)
#endif
        displayedSuggestion=nil
    }
    private func showSuggestion(_ suggestion: String?, client: IMKTextInput) {
        let selection = client.selectedRange()
#if MELTYPE_RUST_CORE
        var caret=NSRect.zero
        if suggestion != nil && selection.location != NSNotFound {
            _=client.attributes(forCharacterIndex:selection.location,lineHeightRectangle:&caret)
        }
        if let panel=NativeCore.shared.editorPolicy?.suggestion(panel:suggestionPanel,text:suggestion,caret:caret,selected:selection.location != NSNotFound) as? NSPanel { suggestionPanel=panel }
#else
        guard let suggestion, selection.location != NSNotFound else {
            suggestionPanel?.orderOut(nil)
            displayedSuggestion = nil
            return
        }
        var caret = NSRect.zero
        _ = client.attributes(forCharacterIndex: selection.location, lineHeightRectangle: &caret)
        guard caret.minX.isFinite, caret.minY.isFinite,
              caret.width.isFinite, caret.height.isFinite, caret.height > 0 else {
            suggestionPanel?.orderOut(nil)
            displayedSuggestion = nil
            return
        }
        displayedSuggestion = suggestion
        let label = NSTextField(labelWithString: suggestion)
        label.font = .systemFont(ofSize: 13)
        label.sizeToFit()
        let size = NSSize(width: label.frame.width + 24, height: label.frame.height + 16)
        let panel = suggestionPanel ?? NSPanel(contentRect: NSRect(origin: .zero, size: size),
            styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.level = .floating
        panel.hasShadow = true
        panel.backgroundColor = .windowBackgroundColor
        panel.ignoresMouseEvents = true
        panel.setContentSize(size)
        panel.contentView?.subviews.forEach { $0.removeFromSuperview() }
        label.frame.origin = NSPoint(x: 12, y: 8)
        panel.contentView?.addSubview(label)
        if let screen = NSScreen.screens.first(where: { $0.frame.intersects(caret) }) ?? NSScreen.main {
            let visible = screen.visibleFrame
            panel.setFrameOrigin(NSPoint(x: max(visible.minX, min(caret.minX, visible.maxX - size.width)),
                y: max(visible.minY, min(caret.minY - size.height - 6, visible.maxY - size.height))))
        }
        suggestionPanel = panel
        panel.orderFrontRegardless()
#endif
    }

    private func addMark(_ style: Int, to text: NSMutableAttributedString, range: NSRange) {
        guard range.length > 0, range.location + range.length <= text.length,
              let marks = mark(forStyle: style, at: range) else { return }
        var attributes: [NSAttributedString.Key: Any] = [:]
        for (key, value) in marks {
            if let name = key as? String {
                attributes[NSAttributedString.Key(name)] = value
            } else if let name = key as? NSAttributedString.Key {
                attributes[name] = value
            }
        }
        text.addAttributes(attributes, range: range)
    }

    /// 候補で少し (1.5 秒) 止まったら、その候補の意味を候補ウィンドウの注釈に出す (Windows 版と同じ)。
#if !MELTYPE_RUST_CORE
    private var meaningKey: String?
#endif
    private func hideCandidatesWindow() {
#if MELTYPE_RUST_CORE
        if let window = candidatesWindow { _ = NativeCore.shared.editorPolicy?.candidateWindow(window, operation: 4) }
#else
        candidatesWindow?.hide()
#endif
    }
    private func clearMeaning() {
#if MELTYPE_RUST_CORE
        _ = NativeCore.shared.editorPolicy?.reserveMeaning(session, index: 0, text: nil)
#else
        meaningKey = nil
#endif
    }

    private func scheduleMeaning(_ view: CompositionView) {
#if MELTYPE_RUST_CORE
        let token = session
        let index = view.selectedIndex
        guard let policy = NativeCore.shared.editorPolicy,
              policy.reserveMeaning(token, index: index, text: view.meaning), let meaning = view.meaning else { return }
        policy.scheduleMeaning { [weak self] in
            guard self != nil, policy.matchesMeaning(token, index: index, text: meaning),
                  let window = candidatesWindow else { return }
            _ = policy.candidateWindow(window, operation: 5, text: meaning)
        }
#else
        let key = view.meaning.map { "\(view.selectedIndex):\($0)" }
        guard key != meaningKey else { return }
        meaningKey = key
        guard let key, let meaning = view.meaning else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            guard let self, self.meaningKey == key, let window = candidatesWindow, window.isVisible() else { return }
            window.showAnnotation(NSAttributedString(string: meaning))
        }
#endif
    }

    private func updateCandidates(_ view: CompositionView) {
        guard let window = candidatesWindow else { return }
#if MELTYPE_RUST_CORE
        var beganSuppression = false
        defer { if beganSuppression { selectingFromCore = false } }
        guard let policy = NativeCore.shared.editorPolicy else { return }
        policy.candidates(view, previous: candidateList, visible: policy.candidateWindow(window, operation: 0) == 1, windowIndex: windowIndex) { operation, argument in
            switch operation {
            case 0: beganSuppression = true; self.selectingFromCore = true
            case 1:
                self.candidateList = view.candidates
                guard policy.candidateWindow(window, operation: 1, argument: Int(kIMKLocateCandidatesBelowHint)) != 0 else { return 0 }
                self.windowIndex = 0
            case 2:
                return policy.candidateWindow(window, operation: 2, argument: argument)
            case 3:
                return policy.candidateWindow(window, operation: 3, argument: argument)
            case 4: self.windowIndex = argument
            case 5: self.scheduleMeaning(view)
            case 6:
                self.clearMeaning(); self.candidateList = []
                return policy.candidateWindow(window, operation: 4)
            default: return -1
            }
            return 1
        }
#else
        if view.candidates.count > 1 {
            // 作り直し・選択で候補ウィンドウから candidateSelectionChanged が来ても、本体に返さない。
            selectingFromCore = true
            defer { selectingFromCore = false }
            // 一覧を作り直すと選択が先頭に戻るので、中身が変わったとき (別の文節に移ったときなど) だけ作り直す。
            if candidateList != view.candidates || !window.isVisible() {
                candidateList = view.candidates
                window.update()
                window.show(kIMKLocateCandidatesBelowHint)
                windowIndex = 0
            }
            if view.selectedIndex >= 0 && view.selectedIndex < view.candidates.count {
                selectInWindow(window, index: view.selectedIndex)
            }
            scheduleMeaning(view)
        } else {
            clearMeaning()
            candidateList = []
            window.hide()
        }
#endif
    }

    /// 候補ウィンドウで今選ばれている候補の番号 (一覧を作り直すと先頭に戻る)。
    private var windowIndex = 0

    /// 候補ウィンドウの選択を index 番目の候補に合わせる。
    /// selectCandidate(withIdentifier:) では選択が動かず、selectedCandidate() も今の選択とずれることがある (macOS 26)。
    /// なので、選択の位置はこちらで覚えておき、矢印キーと同じ moveDown / moveUp でその差だけ動かす。
#if !MELTYPE_RUST_CORE
    private func selectInWindow(_ window: IMKCandidates, index: Int) {
        let step = index > windowIndex ? #selector(NSResponder.moveDown(_:)) : #selector(NSResponder.moveUp(_:))
        guard window.responds(to: step) else { return }
        for _ in 0..<abs(index - windowIndex) {
            window.perform(step, with: nil)
        }
        windowIndex = index
    }

#endif
    /// 入力欄のキャレットの前後の文字列 (それぞれ 20 文字まで)。取れなければ nil。
    private func surroundingText(of client: IMKTextInput) -> (String?, String?) {
        let selection = client.selectedRange()
#if MELTYPE_RUST_CORE
        guard let ranges = NativeCore.shared.editorPolicy?.contextRanges(selection: selection, code: codeInput) else { return (nil, nil) }
        let before = client.attributedSubstring(from: ranges.0)?.string
        let after = client.attributedSubstring(from: ranges.1)?.string
#else
        guard selection.location != NSNotFound else { return (nil, nil) }
        let start = max(0, selection.location - (codeInput ? 4000 : 20))
        let before = client.attributedSubstring(from: NSRange(location: start, length: selection.location - start))?.string
        let after = client.attributedSubstring(from: NSRange(location: selection.location + selection.length, length: 20))?.string
#endif
        return (before, after)
    }
}
