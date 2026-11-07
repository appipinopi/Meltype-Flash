// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation
import CryptoKit

struct AIDataFile: Decodable { let name: String; let url: URL; let sha256: String }
struct AIDataPackage: Decodable { let revision: String; let files: [AIDataFile] }
struct AIPackages: Decodable { let shunti: AIDataPackage }
struct AISelection: Codable { var model = "None" }

// The containing app downloads; the keyboard only reads local files.
final class AISettingsStore {
    let directory: URL?
    init(directory: URL? = AISettingsStore.sharedDirectory) { self.directory = directory }
    static var sharedDirectory: URL? {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "MeltypeAppearanceAppGroup") as? String else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)?.appendingPathComponent("AI", isDirectory: true)
    }
    var dataDirectory: URL? { directory?.appendingPathComponent("shunti-14801bff-s8-data", isDirectory: true) }
    func selection() -> String {
        guard let file = directory?.appendingPathComponent("selection.json"),
              let bytes = try? Data(contentsOf: file), bytes.count < 4096,
              let value = try? JSONDecoder().decode(AISelection.self, from: bytes), ["None", "Shunti"].contains(value.model) else { return "None" }
        return value.model
    }
    func select(_ model: String) throws {
        guard ["None", "Shunti"].contains(model), let directory = directory else { throw CocoaError(.fileWriteNoPermission) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try JSONEncoder().encode(AISelection(model: model)).write(to: directory.appendingPathComponent("selection.json"), options: .atomic)
    }
    var installed: Bool {
        guard let root = dataDirectory else { return false }
        return ["kkc_lex.bin", "kkc_model.bin", "receipt.json"].allSatisfy { FileManager.default.fileExists(atPath: root.appendingPathComponent($0).path) }
    }
    static func hash(_ file: URL) throws -> String {
        let input = try FileHandle(forReadingFrom: file); defer { try? input.close() }
        var hash = SHA256()
        while true { let data = try input.read(upToCount: 65536) ?? Data(); if data.isEmpty { break }; hash.update(data: data) }
        return hash.finalize().map { String(format: "%02x", $0) }.joined()
    }
    func install() async throws {
        guard let directory = directory, let target = dataDirectory,
              let manifest = Bundle.main.url(forResource: "packages", withExtension: "json") else { throw CocoaError(.fileWriteNoPermission) }
        let package = try JSONDecoder().decode(AIPackages.self, from: Data(contentsOf: manifest)).shunti
        guard package.revision == "14801bff-s8", Set(package.files.map { $0.name }) == Set(["kkc_lex.bin", "kkc_model.bin"]) else { throw CocoaError(.fileReadCorruptFile) }
        if FileManager.default.fileExists(atPath: target.path) {
            for file in package.files { guard try Self.hash(target.appendingPathComponent(file.name)) == file.sha256 else { throw CocoaError(.fileReadCorruptFile) } }
            return
        }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let stage = directory.appendingPathComponent("setup-" + UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: stage, withIntermediateDirectories: false)
        defer { try? FileManager.default.removeItem(at: stage) }
        for file in package.files {
            guard file.url.scheme == "https", file.url.host == "github.com", file.sha256.count == 64 else { throw CocoaError(.fileReadCorruptFile) }
            let (temporary, response) = try await URLSession.shared.download(from: file.url)
            defer { try? FileManager.default.removeItem(at: temporary) }
            guard (response as? HTTPURLResponse)?.statusCode == 200,
                  let size = try temporary.resourceValues(forKeys: [.fileSizeKey]).fileSize, size > 0, size <= 128 * 1024 * 1024,
                  try Self.hash(temporary) == file.sha256 else { throw CocoaError(.fileReadCorruptFile) }
            try FileManager.default.moveItem(at: temporary, to: stage.appendingPathComponent(file.name))
        }
        let files = Dictionary(uniqueKeysWithValues: package.files.map { ($0.name, $0.sha256) })
        try JSONSerialization.data(withJSONObject: ["revision":package.revision,"files":files]).write(to: stage.appendingPathComponent("receipt.json"), options: .atomic)
        try FileManager.default.moveItem(at: stage, to: target)
    }
}
