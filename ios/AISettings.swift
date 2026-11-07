// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import Foundation
import CryptoKit

struct AIDataFile: Decodable { let name: String; let url: URL; let sha256: String; let size: Int? }
struct AIDataPackage: Decodable { let revision: String; let files: [AIDataFile] }
struct AIPackages: Decodable { let shunti: AIDataPackage }
struct AISelection: Codable { var model = "None"; var variant: String? }
struct AIVariant: Decodable { let id: String; let family: String; let label: String; let directory: String; let files: [AIDataFile]; let available: Bool }
struct AIModelCatalog: Decodable { let models: [AIVariant] }

// The containing app downloads; the keyboard only reads local files.
final class AISettingsStore {
    let directory: URL?
    init(directory: URL? = AISettingsStore.sharedDirectory) { self.directory = directory }
    static var sharedDirectory: URL? {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "MeltypeAppearanceAppGroup") as? String else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)?.appendingPathComponent("AI", isDirectory: true)
    }
    static let variants: [AIVariant] = {
        guard let file = Bundle(for: AISettingsStore.self).url(forResource: "model-catalog", withExtension: "json"),
              let bytes = try? Data(contentsOf: file), let catalog = try? JSONDecoder().decode(AIModelCatalog.self, from: bytes) else { return [] }
        return catalog.models.filter { $0.family == "Shunti" && $0.available }
    }()
    func variant() -> String {
        guard let file = directory?.appendingPathComponent("selection.json"), let data = try? Data(contentsOf: file), data.count < 4096,
              let value = try? JSONDecoder().decode(AISelection.self, from: data), let id = value.variant,
              Self.variants.contains(where: { $0.id == id }) else { return "shunti-v7-s8p" }
        return id
    }
    func selectVariant(_ id: String) throws {
        guard Self.variants.contains(where: { $0.id == id }), let directory = directory else { throw CocoaError(.fileReadCorruptFile) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try JSONEncoder().encode(AISelection(model: selection(), variant: id)).write(to: directory.appendingPathComponent("selection.json"), options: .atomic)
    }
    var dataDirectory: URL? {
        let folder = Self.variants.first(where: { $0.id == variant() })?.directory ?? "shunti-14801bff-s8"
        return directory?.appendingPathComponent(folder + "-data", isDirectory: true)
    }
    func selection() -> String {
        guard let file = directory?.appendingPathComponent("selection.json"),
              let bytes = try? Data(contentsOf: file), bytes.count < 4096,
              let value = try? JSONDecoder().decode(AISelection.self, from: bytes), ["None", "Shunti"].contains(value.model) else { return "None" }
        return value.model
    }
    func select(_ model: String) throws {
        guard ["None", "Shunti"].contains(model), let directory = directory else { throw CocoaError(.fileWriteNoPermission) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try JSONEncoder().encode(AISelection(model: model, variant: variant())).write(to: directory.appendingPathComponent("selection.json"), options: .atomic)
    }
    var installed: Bool {
        guard let root = dataDirectory else { return false }
        guard let package = Self.variants.first(where: { $0.id == variant() }),
              let data = try? Data(contentsOf: root.appendingPathComponent("receipt.json")),
              let receipt = try? JSONSerialization.jsonObject(with: data) as? [String: Any], let files = receipt["files"] as? [String:String] else { return false }
        return package.files.allSatisfy { file in
            guard files[file.name] == file.sha256, let size = try? root.appendingPathComponent(file.name).resourceValues(forKeys: [.fileSizeKey]).fileSize else { return false }
            return file.size == size
        }
    }
    static func hash(_ file: URL) throws -> String {
        let input = try FileHandle(forReadingFrom: file); defer { try? input.close() }
        var hash = SHA256()
        while true { let data = try input.read(upToCount: 65536) ?? Data(); if data.isEmpty { break }; hash.update(data: data) }
        return hash.finalize().map { String(format: "%02x", $0) }.joined()
    }
    func install() async throws {
        guard let directory = directory, let target = dataDirectory,
              let selected = Self.variants.first(where: { $0.id == variant() }) else { throw CocoaError(.fileWriteNoPermission) }
        let package = AIDataPackage(revision: selected.directory, files: selected.files)
        guard Set(package.files.map { $0.name }) == Set(["kkc_lex.bin", "kkc_model.bin"]) else { throw CocoaError(.fileReadCorruptFile) }
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
