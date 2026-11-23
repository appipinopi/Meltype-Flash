// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

// Shared styling for the containing app; keyboard rendering stays extension-safe.
enum AppStyle {
    static let yellow = UIColor(red: 1, green: 0.86, blue: 0.16, alpha: 1)
    static let ink = UIColor(red: 0.10, green: 0.11, blue: 0.12, alpha: 1)
    static let canvas = UIColor { $0.userInterfaceStyle == .dark
        ? UIColor(red: 0.07, green: 0.08, blue: 0.09, alpha: 1)
        : UIColor(red: 0.96, green: 0.96, blue: 0.94, alpha: 1) }

    static func label(_ text: String, _ style: UIFont.TextStyle = .body, color: UIColor = .label) -> UILabel {
        let label = UILabel()
        label.text = text; label.numberOfLines = 0; label.textColor = color
        label.font = .preferredFont(forTextStyle: style)
        label.adjustsFontForContentSizeCategory = true
        return label
    }
    static func column(spacing: CGFloat = 12) -> UIStackView {
        let stack = UIStackView(); stack.axis = .vertical; stack.spacing = spacing
        return stack
    }
    static func row(_ views: [UIView], spacing: CGFloat = 12) -> UIStackView {
        let row = UIStackView(arrangedSubviews: views); row.spacing = spacing; row.alignment = .center
        return row
    }
    static func symbol(_ name: String, color: UIColor = .label) -> UIImageView {
        let image = UIImageView(image: UIImage(systemName: name))
        image.tintColor = color; image.contentMode = .scaleAspectFit
        image.widthAnchor.constraint(equalToConstant: 22).isActive = true
        image.heightAnchor.constraint(equalToConstant: 22).isActive = true
        image.setContentHuggingPriority(.required, for: .horizontal)
        return image
    }
    static func card(_ content: UIView, color: UIColor = .secondarySystemGroupedBackground, inset: CGFloat = 20) -> UIView {
        let card = UIView(); card.backgroundColor = color
        card.layer.cornerRadius = 24; card.layer.cornerCurve = .continuous
        content.translatesAutoresizingMaskIntoConstraints = false; card.addSubview(content)
        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: card.leadingAnchor, constant: inset),
            content.trailingAnchor.constraint(equalTo: card.trailingAnchor, constant: -inset),
            content.topAnchor.constraint(equalTo: card.topAnchor, constant: inset),
            content.bottomAnchor.constraint(equalTo: card.bottomAnchor, constant: -inset)
        ])
        return card
    }
    static func scrollColumn(in controller: UIViewController) -> UIStackView {
        controller.view.backgroundColor = canvas
        let scroll = UIScrollView(); scroll.keyboardDismissMode = .interactive
        scroll.translatesAutoresizingMaskIntoConstraints = false; controller.view.addSubview(scroll)
        let stack = column(spacing: 18); stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(stack)
        NSLayoutConstraint.activate([
            scroll.leadingAnchor.constraint(equalTo: controller.view.safeAreaLayoutGuide.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: controller.view.safeAreaLayoutGuide.trailingAnchor),
            scroll.topAnchor.constraint(equalTo: controller.view.safeAreaLayoutGuide.topAnchor),
            scroll.bottomAnchor.constraint(equalTo: controller.view.keyboardLayoutGuide.topAnchor),
            stack.leadingAnchor.constraint(equalTo: scroll.contentLayoutGuide.leadingAnchor, constant: 20),
            stack.trailingAnchor.constraint(equalTo: scroll.contentLayoutGuide.trailingAnchor, constant: -20),
            stack.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 12),
            stack.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -28),
            stack.widthAnchor.constraint(equalTo: scroll.frameLayoutGuide.widthAnchor, constant: -40)
        ])
        return stack
    }
    static func button(_ title: String, symbol: String, primary: Bool = false, action: @escaping () -> Void) -> UIButton {
        let button = UIButton(type: .system)
        var config = primary ? UIButton.Configuration.filled() : UIButton.Configuration.tinted()
        config.title = title; config.image = UIImage(systemName: symbol); config.imagePadding = 10
        config.baseBackgroundColor = primary ? yellow : .secondarySystemFill
        config.baseForegroundColor = primary ? ink : .label
        config.cornerStyle = .large
        config.contentInsets = NSDirectionalEdgeInsets(top: 15, leading: 16, bottom: 15, trailing: 16)
        button.configuration = config
        button.heightAnchor.constraint(greaterThanOrEqualToConstant: 50).isActive = true
        button.addAction(UIAction { _ in action() }, for: .touchUpInside)
        return button
    }
}

final class KeyboardPreviewView: UIView {
    private let backdrop = KeyboardBackdrop()
    private var keys: [UILabel] = []
    private let composing = AppStyle.label("今日は good day", .subheadline)
    override init(frame: CGRect) {
        super.init(frame: frame)
        layer.cornerRadius = 20; layer.cornerCurve = .continuous; clipsToBounds = true
        isAccessibilityElement = true
        accessibilityLabel = "キーボードのプレビュー。日本語と英語を混ぜて入力できます。"
        backdrop.translatesAutoresizingMaskIntoConstraints = false; addSubview(backdrop)
        let column = AppStyle.column(spacing: 5)
        column.translatesAutoresizingMaskIntoConstraints = false; addSubview(column)
        NSLayoutConstraint.activate([
            backdrop.leadingAnchor.constraint(equalTo: leadingAnchor), backdrop.trailingAnchor.constraint(equalTo: trailingAnchor),
            backdrop.topAnchor.constraint(equalTo: topAnchor), backdrop.bottomAnchor.constraint(equalTo: bottomAnchor),
            column.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 10), column.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -10),
            column.topAnchor.constraint(equalTo: topAnchor, constant: 12), column.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -10)
        ])
        column.addArrangedSubview(composing)
        column.setCustomSpacing(10, after: composing)
        let top: [String] = "qwertyuiop".map { String($0) }
        let middle: [String] = "asdfghjkl".map { String($0) }
        let bottom: [String] = ["⇧"] + "zxcvbnm".map { String($0) } + ["⌫"]
        let rows: [[String]] = [top, middle, bottom, ["123", "◎", "空白・変換", "確定 ↵"]]
        for letters in rows {
            let row = UIStackView(); row.distribution = .fillEqually; row.spacing = 4
            row.heightAnchor.constraint(equalToConstant: 28).isActive = true
            for text in letters {
                let key = UILabel(); key.text = text; key.textAlignment = .center
                key.font = .systemFont(ofSize: letters.count > 5 ? 13 : 11, weight: .medium)
                key.layer.cornerRadius = 5; key.clipsToBounds = true
                row.addArrangedSubview(key); keys.append(key)
            }
            column.addArrangedSubview(row)
        }
        apply(KeyboardAppearance(), image: nil)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    func apply(_ settings: KeyboardAppearance, image: UIImage?) {
        backdrop.apply(settings.theme, image: image)
        composing.textColor = settings.theme.text
        for key in keys {
            let confirm = key.text == "確定 ↵"
            key.backgroundColor = confirm ? AppStyle.yellow : settings.theme.key
            key.textColor = confirm ? AppStyle.ink : settings.theme.text
        }
    }
}
