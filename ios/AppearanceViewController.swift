// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit
import PhotosUI
import UniformTypeIdentifiers

final class ThemeChoice: UIControl {
    let theme: KeyboardTheme
    private let check = AppStyle.symbol("checkmark.circle.fill", color: AppStyle.ink)
    init(_ theme: KeyboardTheme) {
        self.theme = theme
        super.init(frame: .zero)
        layer.cornerRadius = 16; layer.cornerCurve = .continuous
        let column = AppStyle.column(spacing: 10); column.isUserInteractionEnabled = false
        column.translatesAutoresizingMaskIntoConstraints = false; addSubview(column)
        let swatches = UIStackView(); swatches.spacing = 4; swatches.distribution = .fillEqually
        for _ in 0..<5 {
            let key = UIView(); key.backgroundColor = theme.key; key.layer.cornerRadius = 3
            swatches.addArrangedSubview(key)
        }
        swatches.backgroundColor = theme.background; swatches.isLayoutMarginsRelativeArrangement = true
        swatches.layoutMargins = UIEdgeInsets(top: 6, left: 6, bottom: 6, right: 6)
        swatches.layer.cornerRadius = 7
        swatches.heightAnchor.constraint(equalToConstant: 30).isActive = true
        column.addArrangedSubview(swatches)
        column.addArrangedSubview(AppStyle.row([AppStyle.label(theme.label, .caption1), check], spacing: 4))
        NSLayoutConstraint.activate([
            column.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12), column.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            column.topAnchor.constraint(equalTo: topAnchor, constant: 12), column.bottomAnchor.constraint(equalTo: bottomAnchor, constant: -12)
        ])
        isAccessibilityElement = true; accessibilityLabel = theme.label; accessibilityTraits = .button
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    func select(_ selected: Bool) {
        backgroundColor = selected ? AppStyle.yellow.withAlphaComponent(0.14) : AppStyle.canvas
        layer.borderWidth = selected ? 2 : 0
        layer.borderColor = AppStyle.yellow.cgColor
        check.alpha = selected ? 1 : 0
        check.tintColor = .label
        accessibilityTraits = selected ? [.button, .selected] : [.button]
        accessibilityValue = selected ? "選択中" : ""
    }
}

final class AppearanceViewController: UIViewController, PHPickerViewControllerDelegate {
    private let store = KeyboardAppearanceStore()
    private let status = AppStyle.label("キーボードを開き直すと反映されます。", .footnote, color: .secondaryLabel)
    private let preview = KeyboardPreviewView()
    private var themes: [KeyboardTheme: ThemeChoice] = [:]
    private var choose: UIButton!
    private var remove: UIButton!
    private var importing = false

    override func viewDidLoad() {
        super.viewDidLoad()
        title = "背景・テーマ"
        navigationItem.largeTitleDisplayMode = .never
        let column = AppStyle.scrollColumn(in: self)
        let intro = AppStyle.column(spacing: 6)
        intro.addArrangedSubview(AppStyle.label("あなたの色で、\nあなたの言葉を。", .title1))
        intro.addArrangedSubview(AppStyle.label("毎日ふれる場所を、好きなデザインに。", .subheadline, color: .secondaryLabel))
        column.addArrangedSubview(intro)
        column.addArrangedSubview(preview)
        let colors = AppStyle.column(spacing: 14)
        colors.addArrangedSubview(AppStyle.row([AppStyle.symbol("circle.lefthalf.filled"), AppStyle.label("カラーを選ぶ", .headline)]))
        let all = KeyboardTheme.allCases
        for start in stride(from: 0, to: all.count, by: 2) {
            let row = UIStackView(); row.spacing = 10; row.distribution = .fillEqually; row.alignment = .fill
            for theme in all[start..<min(start + 2, all.count)] {
                let button = ThemeChoice(theme)
                button.addAction(UIAction { [weak self] _ in
                    guard let self = self else { return }
                    self.perform { try self.store.setTheme(theme) }
                }, for: .touchUpInside)
                row.addArrangedSubview(button); themes[theme] = button
            }
            colors.addArrangedSubview(row)
        }
        column.addArrangedSubview(AppStyle.card(colors))
        let photo = AppStyle.column(spacing: 12)
        photo.addArrangedSubview(AppStyle.row([AppStyle.symbol("photo.on.rectangle.angled"), AppStyle.label("お気に入りの一枚を", .headline)]))
        photo.addArrangedSubview(AppStyle.label("写真をキーボードの背景に。画像はこの端末内に保存されます。", .subheadline, color: .secondaryLabel))
        choose = AppStyle.button("写真から選ぶ", symbol: "plus", primary: true) { [weak self] in self?.choosePhoto() }
        choose.accessibilityIdentifier = "choose-background"
        photo.addArrangedSubview(choose)
        remove = AppStyle.button("背景画像を外す", symbol: "trash") { [weak self] in
            guard let self = self else { return }
            self.perform { try self.store.removeImage() }
        }
        photo.addArrangedSubview(remove)
        column.addArrangedSubview(AppStyle.card(photo))
        status.accessibilityIdentifier = "appearance-status"; column.addArrangedSubview(status)
        refresh()
        if store.directory == nil {
            status.text = "背景設定を共有できません。本体アプリとキーボードのApp Groupsの署名設定を確認してください。"
            status.textColor = .systemRed
            choose.isEnabled = false; remove.isEnabled = false; themes.values.forEach { $0.isEnabled = false }
        }
    }
    private func choosePhoto() {
        var configuration = PHPickerConfiguration(); configuration.filter = .images; configuration.selectionLimit = 1
        let picker = PHPickerViewController(configuration: configuration); picker.delegate = self
        present(picker, animated: true)
    }
    private func refresh() {
        let settings = store.load()
        preview.apply(settings, image: store.image(for: settings))
        for (theme, button) in themes { button.select(settings.theme == theme) }
        remove?.isHidden = settings.imageName == nil
    }
    private func perform(_ operation: () throws -> Void) {
        do {
            try operation(); refresh()
            status.text = "保存しました。キーボードを開き直すと反映されます。"; status.textColor = .secondaryLabel
            UIAccessibility.post(notification: .announcement, argument: "保存しました")
        } catch { status.text = "保存できませんでした。署名設定や空き容量を確認してください。"; status.textColor = .systemRed }
    }
    func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
        picker.dismiss(animated: true)
        guard !importing, let provider = results.first?.itemProvider else { return }
        importing = true; choose.isEnabled = false; remove.isEnabled = false; status.text = "画像を準備中…"
        provider.loadFileRepresentation(forTypeIdentifier: UTType.image.identifier) { [weak self] file, _ in
            let result = Result<Data, Error> {
                guard let file = file else { throw CocoaError(.fileReadCorruptFile) }
                return try KeyboardAppearanceStore.prepareImage(from: file)
            }
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.importing = false; self.choose.isEnabled = true; self.remove.isEnabled = true
                switch result {
                case .success(let data): self.perform { try self.store.installImage(data) }
                case .failure: self.status.text = "画像を読み込めませんでした。別の画像を選んでください。"; self.status.textColor = .systemRed
                }
            }
        }
    }
    override func traitCollectionDidChange(_ previousTraitCollection: UITraitCollection?) {
        super.traitCollectionDidChange(previousTraitCollection)
        if isViewLoaded { refresh() }
    }
}
