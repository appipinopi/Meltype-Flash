// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation
import Aptabase

/// 利用状況の集計 (Aptabase)。
///
/// 送るのは出来事の名前と、Aptabaseが付ける端末の種類・OSの版・アプリの版・言語。
/// 入力内容・入力履歴・辞書・設定値・入力欄の文字列は送らない。
/// 既定は有効で、アプリの画面で切った人だけ送らない。キーボード拡張では初期化しない。
/// 呼び出しはメインスレッドからだけ行う。
enum FlashAnalytics {
    /// Aptabaseのアプリキー (A-US-... はUSリージョン)。
    static let appKey = "A-US-4882313783"
    /// 全員から既定で送る (設定画面で切れる)。false にすると既定で送らない。
    private static let defaultEnabled = true
    private static let enabledKey = "MeltypeFlashAnalytics"
    private static var started = false

    static var isEnabled: Bool {
        UserDefaults.standard.object(forKey: enabledKey) as? Bool ?? defaultEnabled
    }

    /// 入りにしたときはすぐ送り始め、切ったときは次の起動から送らない。
    static func setEnabled(_ enabled: Bool) {
        UserDefaults.standard.set(enabled, forKey: enabledKey)
        if enabled { start() }
    }

    /// 起動時に一度だけ呼ぶ。無効なら何もしない。
    static func start() {
        guard isEnabled, !started else { return }
        started = true
        Aptabase.shared.initialize(appKey: appKey)
    }

    /// 出来事の名前だけを送る。プロパティは付けない。
    static func track(_ event: String) {
        guard started, isEnabled else { return }
        Aptabase.shared.trackEvent(event)
    }
}
