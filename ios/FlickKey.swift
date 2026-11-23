// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

enum FlickKana {
    // Center, left, up, right, down. Keep every direction explicit, including や / わ.
    static let groups = [
        ["あ", "い", "う", "え", "お"], ["か", "き", "く", "け", "こ"], ["さ", "し", "す", "せ", "そ"],
        ["た", "ち", "つ", "て", "と"], ["な", "に", "ぬ", "ね", "の"], ["は", "ひ", "ふ", "へ", "ほ"],
        ["ま", "み", "む", "め", "も"], ["や", "「", "ゆ", "」", "よ"], ["ら", "り", "る", "れ", "ろ"],
        ["わ", "を", "ん", "ー", "～"], ["、", "。", "？", "！", "…"]
    ]
    static func direction(_ offset: CGPoint, threshold: CGFloat = 12) -> Int {
        if max(abs(offset.x), abs(offset.y)) < threshold { return 0 }
        if abs(offset.x) > abs(offset.y) { return offset.x < 0 ? 1 : 3 }
        return offset.y < 0 ? 2 : 4
    }
    static func variant(_ character: String) -> String? {
        let cycles = ["あぁ", "いぃ", "うぅゔ", "えぇ", "おぉ", "かが", "きぎ", "くぐ", "けげ", "こご",
                      "さざ", "しじ", "すず", "せぜ", "そぞ", "ただ", "ちぢ", "つっづ", "てで", "とど",
                      "はばぱ", "ひびぴ", "ふぶぷ", "へべぺ", "ほぼぽ", "やゃ", "ゆゅ", "よょ", "わゎ"]
        for cycle in cycles {
            let chars = cycle.map { String($0) }
            if let index = chars.firstIndex(of: character) { return chars[(index + 1) % chars.count] }
        }
        return nil
    }
}

final class FlickKey: UIButton {
    let kana: [String]
    private let hint = UILabel()
    private var origin: CGPoint?
    private var choice = 0
    private let input: (String) -> Void
    init(kana: [String], input: @escaping (String) -> Void) {
        self.kana = kana; self.input = input
        super.init(frame: .zero)
        setTitle(kana[0], for: .normal)
        titleLabel?.font = .systemFont(ofSize: 27, weight: .medium)
        hint.font = .systemFont(ofSize: 10); hint.textAlignment = .center
        hint.text = kana.dropFirst().joined(separator: " "); hint.isUserInteractionEnabled = false
        addSubview(hint)
        accessibilityLabel = kana[0] + "行"
        accessibilityHint = "タップで\(kana[0])。左\(kana[1])、上\(kana[2])、右\(kana[3])、下\(kana[4])。"
        accessibilityIdentifier = "flick-" + kana[0]
        accessibilityCustomActions = kana.dropFirst().map { character in
            UIAccessibilityCustomAction(name: character + "を入力") { [weak self] _ in
                self?.input(character); return true
            }
        }
        addAction(UIAction { [weak self] _ in
            guard let self = self else { return }
            let text = self.kana[self.choice]
            self.resetSelection(); self.input(text)
        }, for: [.touchUpInside, .touchUpOutside])
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    override var isHighlighted: Bool { didSet { alpha = isHighlighted ? 0.65 : 1 } }
    override func layoutSubviews() {
        super.layoutSubviews()
        titleLabel?.frame = CGRect(x: 0, y: 3, width: bounds.width, height: max(30, bounds.height - 22))
        titleLabel?.textAlignment = .center
        hint.frame = CGRect(x: 2, y: bounds.height - 19, width: bounds.width - 4, height: 14)
    }
    func setHintColor(_ color: UIColor) { hint.textColor = color.withAlphaComponent(0.65) }
    func updateSelection(offset: CGPoint) {
        choice = FlickKana.direction(offset)
        setTitle(kana[choice], for: .normal)
        accessibilityValue = kana[choice]
    }
    private func resetSelection() {
        origin = nil; choice = 0; isHighlighted = false
        setTitle(kana[0], for: .normal); accessibilityValue = nil
    }
    override func beginTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        resetSelection(); origin = touch.location(in: self); isHighlighted = true
        return true
    }
    override func continueTracking(_ touch: UITouch, with event: UIEvent?) -> Bool {
        if let origin = origin {
            let point = touch.location(in: self)
            updateSelection(offset: CGPoint(x: point.x - origin.x, y: point.y - origin.y))
        }
        return true
    }
    override func endTracking(_ touch: UITouch?, with event: UIEvent?) {
        if let touch = touch, let origin = origin {
            let point = touch.location(in: self)
            updateSelection(offset: CGPoint(x: point.x - origin.x, y: point.y - origin.y))
        }
        // UIControl dispatches touchUpInside / touchUpOutside after tracking ends.
        super.endTracking(touch, with: event)
    }
    override func cancelTracking(with event: UIEvent?) {
        super.cancelTracking(with: event); resetSelection()
    }
    override func accessibilityActivate() -> Bool { resetSelection(); input(kana[0]); return true }
}
