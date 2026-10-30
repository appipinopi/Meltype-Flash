// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit

struct EditingSelection {
    let text: String
    let document: UUID
    let before: String?
    let after: String?
    init?(_ proxy: UITextDocumentProxy) {
        guard let text = proxy.selectedText, !text.isEmpty else { return nil }
        self.text = text; document = proxy.documentIdentifier
        before = proxy.documentContextBeforeInput; after = proxy.documentContextAfterInput
    }
    func matches(_ proxy: UITextDocumentProxy) -> Bool {
        document == proxy.documentIdentifier && text == proxy.selectedText
            && before == proxy.documentContextBeforeInput && after == proxy.documentContextAfterInput
    }
}
