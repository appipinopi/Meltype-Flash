// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

// The containing app owns these text views, so its standard selection menu can
// include custom actions. Other apps keep ownership of their own edit menus.
final class MeltypeTextView: UITextView {
    var canConvertSelection: (() -> Bool)?
    var cancelPendingInput: (() -> Void)?
    var hasPendingInput: (() -> Bool)?
    private var previousLegacyItems: [UIMenuItem]?

    func selectionMenu(suggestedActions: [UIMenuElement]) -> UIMenu {
        var actions: [UIMenuElement] = []
        let enabled = markedTextRange == nil && canConvertSelection?() != false
        if selectedRange.length > 0 {
            actions.append(UIAction(title: "Meltypeで変換", image: UIImage(systemName: "character.cursor.ibeam"), attributes: enabled ? [] : .disabled) { [weak self] _ in self?.meltypeConvertSelection() })
            actions.append(UIAction(title: "選択範囲を削除", image: UIImage(systemName: "delete.left"), attributes: enabled ? .destructive : [.destructive, .disabled]) { [weak self] _ in self?.meltypeDeleteSelection() })
        }
        if hasPendingInput?() == true {
            actions.append(UIAction(title: "未確定を取り消す") { [weak self] _ in self?.cancelPendingInput?() })
        }
        // Keep the system's Copy, Cut, Paste, Select All and other native actions.
        return UIMenu(children: actions + suggestedActions)
    }
    override func becomeFirstResponder() -> Bool {
        let wasFirst = isFirstResponder
        let became = super.becomeFirstResponder()
        if became && !wasFirst {
            if #available(iOS 16.0, *) { } else {
                previousLegacyItems = UIMenuController.shared.menuItems
                UIMenuController.shared.menuItems = [
                    UIMenuItem(title: "Meltypeで変換", action: #selector(meltypeConvertSelection)),
                    UIMenuItem(title: "選択範囲を削除", action: #selector(meltypeDeleteSelection)),
                    UIMenuItem(title: "未確定を取り消す", action: #selector(meltypeCancelComposition))
                ]
            }
        }
        return became
    }
    override func resignFirstResponder() -> Bool {
        let resigned = super.resignFirstResponder()
        if resigned {
            if #available(iOS 16.0, *) { } else { UIMenuController.shared.menuItems = previousLegacyItems }
        }
        return resigned
    }
    override func canPerformAction(_ action: Selector, withSender sender: Any?) -> Bool {
        if action == #selector(meltypeConvertSelection) || action == #selector(meltypeDeleteSelection) {
            return selectedRange.length > 0 && markedTextRange == nil && canConvertSelection?() != false
        }
        if action == #selector(meltypeCancelComposition) { return hasPendingInput?() == true }
        return super.canPerformAction(action, withSender: sender)
    }
    @objc private func meltypeCancelComposition() { cancelPendingInput?() }
    @objc private func meltypeDeleteSelection() {
        guard selectedRange.length > 0, markedTextRange == nil, canConvertSelection?() != false else { return }
        deleteBackward()
    }
    @objc private func meltypeConvertSelection() {
        guard selectedRange.length > 0, markedTextRange == nil, canConvertSelection?() != false,
              let owner = owningController else { return }
        guard let conversion = SelectionMenuConversion(field: self), !conversion.candidates.isEmpty else {
            let notice = UIAlertController(title: "変換できませんでした", message: "読みを取得できない文字か、長すぎる範囲です。元の文字は変更していません。", preferredStyle: .alert)
            notice.addAction(UIAlertAction(title: "閉じる", style: .default)); owner.present(notice, animated: true)
            return
        }
        let menu = UIAlertController(title: "Meltypeで変換", message: "候補を選ぶと選択範囲だけを置き換えます。", preferredStyle: .actionSheet)
        for (index, candidate) in conversion.candidates.prefix(20).enumerated() {
            menu.addAction(UIAlertAction(title: candidate, style: .default) { _ in conversion.commit(index: index) })
        }
        menu.addAction(UIAlertAction(title: "キャンセル", style: .cancel))
        if let popover = menu.popoverPresentationController {
            popover.sourceView = self
            popover.sourceRect = selectedTextRange.map { firstRect(for: $0) } ?? bounds
        }
        owner.present(menu, animated: true)
    }
    private var owningController: UIViewController? {
        var responder: UIResponder? = self
        while let current = responder {
            if let controller = current as? UIViewController { return controller }
            responder = current.next
        }
        return nil
    }
}

private final class SelectionMenuConversion {
    private weak var field: UITextView?
    private let original: String
    private let selection: NSRange
    private let handle: UInt64
    let candidates: [String]

    init?(field: UITextView) {
        let whole = field.text ?? ""
        let range = field.selectedRange
        guard range.location <= whole.utf16.count, range.length > 0,
              range.length <= whole.utf16.count - range.location else { return nil }
        let text = (whole as NSString).substring(with: range)
        guard text.utf8.count <= 4096 else { return nil }
        let handle = meltype_ios_create()
        guard handle != 0 else { return nil }
        let reply = text.withCString { Self.read(meltype_ios_reconvert(handle, $0, UInt32(text.utf8.count))) }
        guard let reply = reply, reply.consumed, reply.selectionExpect == text,
              reply.commits.isEmpty, let view = reply.view, !view.candidates.isEmpty else {
            meltype_ios_destroy(handle); return nil
        }
        self.field = field; original = whole; selection = range
        self.handle = handle; candidates = view.candidates
    }
    deinit { meltype_ios_destroy(handle) }
    private static func read(_ pointer: UnsafeMutablePointer<CChar>?) -> EngineReply? {
        guard let pointer = pointer else { return nil }
        defer { meltype_ios_free(pointer) }
        return try? JSONDecoder().decode(EngineReply.self, from: Data(String(cString: pointer).utf8))
    }
    func commit(index: Int) {
        guard let field = field, field.window != nil, field.text == original,
              NSEqualRanges(field.selectedRange, selection), field.markedTextRange == nil,
              let selected = Self.read(meltype_ios_select(handle, UInt32(index))),
              let confirmed = Self.read(meltype_ios_commit(handle)) else { return }
        let edits = selected.commits + confirmed.commits
        guard !edits.isEmpty, edits.allSatisfy({ $0.deleteBefore == 0 }) else { return }
        field.insertText(edits.map { $0.text }.joined())
    }
}
