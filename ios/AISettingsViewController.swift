// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

final class AISettingsViewController: UIViewController {
    private let store = AISettingsStore()
    private let choice = UIButton(type: .system)
    private let install = UIButton(type: .system)
    private let status = UILabel()
    private let details = UIStackView()
    private var busy = false
    override func viewDidLoad() {
        super.viewDidLoad(); title = "AIモデル"; view.backgroundColor = AppStyle.canvas
        let column = AppStyle.scrollColumn(in: self)
        choice.showsMenuAsPrimaryAction = true; choice.accessibilityIdentifier = "ai-model"
        column.addArrangedSubview(AppStyle.row([AppStyle.label("AIモデル", .headline), choice]))
        details.axis = .vertical; details.spacing = 12
        details.addArrangedSubview(AppStyle.label("辞書・モデルを別ダウンロードして、端末内で変換します。パス入力は不要です。", .body))
        details.addArrangedSubview(AppStyle.label("共有モデルをキーボードから読むには、iOSの設定でフルアクセスを許可してください。変換時のネットワーク送信はありません。", .footnote, color: .secondaryLabel))
        install.setTitle("shuntiをインストール", for: .normal); install.accessibilityIdentifier = "ai-install"
        install.addTarget(self, action: #selector(confirm), for: .touchUpInside)
        details.addArrangedSubview(install); column.addArrangedSubview(details)
        status.numberOfLines = 0; status.font = .preferredFont(forTextStyle: .footnote); column.addArrangedSubview(status)
        refresh()
    }
    private func refresh() {
        let selected = store.selection()
        choice.setTitle(selected == "Shunti" ? "shunti" : "使用しない", for: .normal)
        choice.menu = UIMenu(children: [("None","使用しない"),("Shunti","shunti")].map { model, label in
            UIAction(title: label, state: selected == model ? .on : .off) { [weak self] _ in
                guard let self = self else { return }
                do { try self.store.select(model); self.refresh() } catch { self.status.text = "共有フォルダーを利用できません。App Groupを含む署名で導入してください。" }
            }
        })
        details.isHidden = selected != "Shunti"
        choice.isEnabled = !busy; install.isEnabled = !busy
        status.text = selected == "Shunti" ? (store.installed ? "導入済みです。キーボードを開き直すと反映します。" : "未導入です。インストールを押してください。") : "AI変換は使用しません。導入済みのデータは保持します。"
    }
    @objc private func confirm() {
        let alert = UIAlertController(title: "shuntiの環境を作成してよいですか？", message: "辞書（CC BY-SA 4.0）・モデル（CC BY 4.0）を任意ダウンロードします。入力内容は送信しません。", preferredStyle: .alert)
        alert.addAction(UIAlertAction(title: "いいえ", style: .cancel))
        alert.addAction(UIAlertAction(title: "はい、作成する", style: .default) { [weak self] _ in self?.start() })
        present(alert, animated: true)
    }
    private func start() {
        busy = true; refresh(); status.text = "AI環境を準備しています…"
        Task { [weak self] in
            guard let self = self else { return }
            do { try await self.store.install(); self.busy = false; self.refresh() }
            catch { self.busy = false; self.refresh(); self.status.text = "AI環境を導入できませんでした: \(error.localizedDescription)" }
        }
    }
}
