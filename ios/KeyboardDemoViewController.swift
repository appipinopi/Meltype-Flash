// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

// This demo uses the same controller and engine as the system keyboard.
private final class DemoDocument: NSObject, UITextDocumentProxy {
    weak var field: UITextView?
    var text: String { get { field?.text ?? "" } set { field?.text = newValue } }
    private var range: NSRange {
        let range = field?.selectedRange ?? NSRange(location: text.utf16.count, length: 0)
        return range.location <= text.utf16.count && range.length <= text.utf16.count - range.location
            ? range : NSRange(location: text.utf16.count, length: 0)
    }
    var documentContextBeforeInput: String? { (text as NSString).substring(to: range.location) }
    var documentContextAfterInput: String? { (text as NSString).substring(from: NSMaxRange(range)) }
    var selectedText: String? { range.length > 0 ? (text as NSString).substring(with: range) : nil }
    var documentInputMode: UITextInputMode? { nil }
    let documentIdentifier = UUID()
    var hasText: Bool { !text.isEmpty }
    func insertText(_ value: String) {
        let range = self.range
        text = (text as NSString).replacingCharacters(in: range, with: value)
        field?.selectedRange = NSRange(location: range.location + value.utf16.count, length: 0)
    }
    func deleteBackward() {
        let range = self.range
        if range.length > 0 { insertText(""); return }
        let before = (text as NSString).substring(to: range.location)
        guard let last = before.last else { return }
        let count = String(last).utf16.count
        field?.selectedRange = NSRange(location: range.location - count, length: count)
        insertText("")
    }
    func adjustTextPosition(byCharacterOffset offset: Int) { }
    func setMarkedText(_ markedText: String, selectedRange: NSRange) { }
    func unmarkText() { }
}
private final class DemoKeyboard: KeyboardViewController {
    let document = DemoDocument()
    override var textDocumentProxy: UITextDocumentProxy { document }
    override var clipboardAccessAllowed: Bool { true }
    override var initialFlickLayout: Bool? { true }
    override var allowsSystemKeyboardSwitch: Bool { false }
}
final class KeyboardDemoViewController: UIViewController, UITextViewDelegate, UIGestureRecognizerDelegate {
    private let keyboard = DemoKeyboard()
    private let flickPan = UIPanGestureRecognizer()
    override func viewDidLoad() {
        super.viewDidLoad(); title = "フリックを試す"
        let column = AppStyle.scrollColumn(in: self)
        if let scroll = column.superview as? UIScrollView {
            scroll.canCancelContentTouches = false; scroll.delaysContentTouches = false
        }
        column.addArrangedSubview(AppStyle.label("タップで「あ」、左へ「い」。", .title2))
        column.addArrangedSubview(AppStyle.label("かなキーを上下左右へフリック。入力した文字を長押しすると、コピーやMeltypeでの変換を使えます。", .subheadline, color: .secondaryLabel))
        let field = MeltypeTextView(); field.font = .preferredFont(forTextStyle: .body)
        field.adjustsFontForContentSizeCategory = true; field.isEditable = true
        field.backgroundColor = .secondarySystemGroupedBackground; field.layer.cornerRadius = 14
        field.textContainerInset = UIEdgeInsets(top: 12, left: 12, bottom: 12, right: 12)
        field.heightAnchor.constraint(equalToConstant: 86).isActive = true
        field.accessibilityIdentifier = "flick-demo-output"; field.accessibilityLabel = "確定した文字"
        keyboard.document.field = field; field.delegate = self
        field.inputView = UIView(frame: CGRect(x: 0, y: 0, width: 1, height: 1))
        field.canConvertSelection = { [weak self] in self?.keyboard.hasPendingComposition != true }
        field.hasPendingInput = { [weak self] in self?.keyboard.hasPendingComposition == true }
        field.cancelPendingInput = { [weak self] in self?.keyboard.performEditingCommand("変換取消") }
        column.addArrangedSubview(field)
        addChild(keyboard)
        column.addArrangedSubview(keyboard.view); keyboard.didMove(toParent: self)
        column.addArrangedSubview(AppStyle.label("アプリ内のお試し入力です。ほかのアプリで使うには、設定からキーボードを追加してください。", .footnote, color: .secondaryLabel))
    }
    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        configureFlickPriority()
    }
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        configureFlickPriority()
    }
    private func configureFlickPriority() {
        if #available(iOS 26.0, *), navigationController?.interactiveContentPopGestureRecognizer != nil {
            if flickPan.view == nil {
                flickPan.delegate = self
                flickPan.cancelsTouchesInView = false
                flickPan.delaysTouchesBegan = false
                flickPan.delaysTouchesEnded = false
                keyboard.view.addGestureRecognizer(flickPan)
            }
            flickPan.isEnabled = true
        }
    }
    override func viewDidDisappear(_ animated: Bool) {
        super.viewDidDisappear(animated)
        flickPan.isEnabled = false
    }
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        var target = touch.view
        while let current = target, current !== keyboard.view {
            if current is FlickKey { return true }
            target = current.superview
        }
        return false
    }
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        if #available(iOS 26.0, *), other === navigationController?.interactiveContentPopGestureRecognizer { return false }
        return true
    }
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldBeRequiredToFailBy other: UIGestureRecognizer) -> Bool {
        // キー上のフリックを優先し、画面全体の「戻る」操作とは競合させない。
        if #available(iOS 26.0, *) { return other === navigationController?.interactiveContentPopGestureRecognizer }
        return false
    }
    @available(iOS 16.0, *)
    func textView(_ textView: UITextView, editMenuForTextIn range: NSRange, suggestedActions: [UIMenuElement]) -> UIMenu? {
        (textView as? MeltypeTextView)?.selectionMenu(for: range, suggestedActions: suggestedActions)
    }
    func textViewDidChangeSelection(_ textView: UITextView) {
        keyboard.selectionWillChange(nil); keyboard.selectionDidChange(nil)
    }

}
