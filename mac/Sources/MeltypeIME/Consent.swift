// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation

/// 初回起動の同意。同意するまで入力メソッドを開始しない。
enum ConsentStore {
    private static let key = "MeltypeConsentVersion"
    static let version = 1
    static var accepted: Bool { UserDefaults.standard.integer(forKey: key) >= version }
    static func accept() { UserDefaults.standard.set(version, forKey: key) }
    static let text = """
    Meltype-FLASH を使うには、次の内容に同意してください。

    ・本ソフトは GNU GPL v3 以降で提供され、無保証です。
    ・より良い入力のために、利用状況（アプリの版・OSの版・起動回数）を匿名で送信します。この送信は設定で停止できません。
    ・入力内容・入力履歴・辞書・設定値・入力欄の文字列は送信しません。
    ・送信先は Aptabase です。

    同意しない場合は「同意しない」を押してください。
    """
}
