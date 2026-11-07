// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import XCTest

final class AISettingsTests: XCTestCase {
    func testSelectionIsOptionalAndDoesNotCreateOrRemoveModels() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = AISettingsStore(directory: directory)
        XCTAssertEqual(store.selection(), "None")
        XCTAssertFalse(store.installed)
        try store.select("Shunti")
        XCTAssertEqual(store.selection(), "Shunti")
        XCTAssertFalse(store.installed)
        let root = try XCTUnwrap(store.dataDirectory)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let file = root.appendingPathComponent("kkc_model.bin")
        try Data("preserve local data".utf8).write(to: file)
        try store.select("None")
        XCTAssertTrue(FileManager.default.fileExists(atPath: file.path))
        XCTAssertThrowsError(try store.select("Zenz"))
        XCTAssertEqual(store.selection(), "None")
        try Data("invalid json".utf8).write(to: directory.appendingPathComponent("selection.json"))
        XCTAssertEqual(store.selection(), "None")
    }
    func testModelVersionsPersistSeparatelyAndChangingSelectionPreservesDownloads() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = AISettingsStore(directory: directory)
        XCTAssertEqual(AISettingsStore.variants.count, 6)
        try store.select("Shunti")
        let original = try XCTUnwrap(store.dataDirectory)
        try FileManager.default.createDirectory(at: original, withIntermediateDirectories: true)
        try Data("preserve".utf8).write(to: original.appendingPathComponent("kkc_model.bin"))
        try store.selectVariant("shunti-v7-xs5")
        XCTAssertEqual(store.variant(), "shunti-v7-xs5")
        XCTAssertNotEqual(store.dataDirectory, original)
        XCTAssertFalse(store.installed)
        try store.select("None"); try store.select("Shunti")
        XCTAssertEqual(store.variant(), "shunti-v7-xs5")
        try store.selectVariant("shunti-v7-s8p")
        XCTAssertEqual(store.dataDirectory, original)
        XCTAssertTrue(FileManager.default.fileExists(atPath: original.appendingPathComponent("kkc_model.bin").path))
        XCTAssertFalse(store.installed) // モデルだけで受領記録がない環境は導入済みにしない。
        XCTAssertThrowsError(try store.selectVariant("../outside"))
        XCTAssertEqual(store.variant(), "shunti-v7-s8p")
    }
    func testMissingSharedContainerCannotEnableAI() {
        let store = AISettingsStore(directory: nil)
        XCTAssertThrowsError(try store.select("Shunti"))
        XCTAssertEqual(store.selection(), "None")
        XCTAssertFalse(store.installed)
    }
}
