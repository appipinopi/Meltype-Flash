// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation
import Aptabase

/// 利用状況の集計 (Aptabase)。
///
/// 送るのは出来事の名前と、Aptabaseが付ける端末の種類・OSの版・アプリの版・言語。
/// 入力内容・入力履歴・辞書・設定値・入力欄の文字列は送らない。
/// 初回起動の同意が済んだ人だけが使い、送信は停止できない。呼び出しはメインスレッドからだけ行う。
enum FlashAnalytics {
    /// Aptabaseのアプリキー (A-US-... はUSリージョン)。
    static let appKey = "A-US-4882313783"
    private static var started = false

    /// 起動時に一度だけ呼ぶ。同意していないときは呼ばない。
    static func start() {
        guard !started else { return }
        started = true
        Aptabase.shared.initialize(appKey: appKey)
    }

    /// 出来事の名前だけを送る。プロパティは付けない。
    static func track(_ event: String) {
        guard started else { return }
        Aptabase.shared.trackEvent(event)
    }
}
