// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
import UIKit
import ImageIO

enum KeyboardTheme: String, Codable, CaseIterable {
    case system, light, dark, blue, pink, mint
    var label: String {
        switch self {
        case .system: return "システムに合わせる"
        case .light: return "ライト"
        case .dark: return "ダーク"
        case .blue: return "ブルー"
        case .pink: return "ピンク"
        case .mint: return "ミント"
        }
    }
    var background: UIColor {
        switch self {
        case .system: return .systemGray5
        case .light: return UIColor(white: 0.91, alpha: 1)
        case .dark: return UIColor(white: 0.12, alpha: 1)
        case .blue: return UIColor(red: 0.86, green: 0.92, blue: 0.99, alpha: 1)
        case .pink: return UIColor(red: 0.98, green: 0.88, blue: 0.93, alpha: 1)
        case .mint: return UIColor(red: 0.86, green: 0.95, blue: 0.90, alpha: 1)
        }
    }
    var key: UIColor { self == .system ? .secondarySystemBackground : self == .dark ? UIColor(white: 0.22, alpha: 1) : .white }
    var text: UIColor { self == .system ? .label : self == .dark ? .white : UIColor(white: 0.10, alpha: 1) }
    func isDark(_ traits: UITraitCollection) -> Bool { self == .dark || self == .system && traits.userInterfaceStyle == .dark }
}

struct KeyboardAppearance: Codable, Equatable {
    var theme: KeyboardTheme = .system
    var imageName: String?
}

// The containing app writes; the keyboard only reads. No keystrokes enter this store.
final class KeyboardAppearanceStore {
    let directory: URL?
    init(directory: URL? = KeyboardAppearanceStore.sharedDirectory) { self.directory = directory }
    static var sharedDirectory: URL? {
        guard let group = Bundle.main.object(forInfoDictionaryKey: "MeltypeAppearanceAppGroup") as? String,
              !group.isEmpty else { return nil }
        return FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group)?
            .appendingPathComponent("KeyboardAppearance", isDirectory: true)
    }
    private func imageURL(_ name: String?) -> URL? {
        guard let name = name, name.hasSuffix(".jpg"), UUID(uuidString: String(name.dropLast(4))) != nil else { return nil }
        return directory?.appendingPathComponent(name)
    }
    func load() -> KeyboardAppearance {
        guard let file = directory?.appendingPathComponent("appearance.json"),
              let data = try? Data(contentsOf: file), data.count <= 4096,
              var settings = try? JSONDecoder().decode(KeyboardAppearance.self, from: data) else { return KeyboardAppearance() }
        if imageURL(settings.imageName) == nil { settings.imageName = nil }
        return settings
    }
    func image(for settings: KeyboardAppearance) -> UIImage? {
        guard let file = imageURL(settings.imageName) else { return nil }
        return UIImage(contentsOfFile: file.path)
    }
    func save(_ settings: KeyboardAppearance) throws {
        guard let directory = directory else { throw CocoaError(.fileWriteNoPermission) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try JSONEncoder().encode(settings).write(to: directory.appendingPathComponent("appearance.json"), options: .atomic)
    }
    func setTheme(_ theme: KeyboardTheme) throws {
        var settings = load(); settings.theme = theme; try save(settings)
    }
    // Decode only a thumbnail, including EXIF rotation. Do this on the settings worker.
    static func prepareImage(from file: URL) throws -> Data {
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithURL(file as CFURL, options),
              let thumbnail = CGImageSourceCreateThumbnailAtIndex(source, 0, [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: 1280,
                kCGImageSourceShouldCacheImmediately: true
              ] as CFDictionary),
              let data = UIImage(cgImage: thumbnail).jpegData(compressionQuality: 0.88) else { throw CocoaError(.fileReadCorruptFile) }
        return data
    }
    func installImage(_ data: Data) throws {
        guard let directory = directory else { throw CocoaError(.fileWriteNoPermission) }
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        var settings = load()
        let previous = imageURL(settings.imageName)
        let name = UUID().uuidString + ".jpg"
        let file = directory.appendingPathComponent(name)
        try data.write(to: file, options: .atomic)
        settings.imageName = name
        do { try save(settings) } catch { try? FileManager.default.removeItem(at: file); throw error }
        if let previous = previous { try? FileManager.default.removeItem(at: previous) }
    }
    func removeImage() throws {
        var settings = load(); let previous = imageURL(settings.imageName)
        settings.imageName = nil; try save(settings)
        if let previous = previous { try? FileManager.default.removeItem(at: previous) }
    }
}

// Reused by the keyboard and preview. No image decoding on layout or key presses.
final class KeyboardBackdrop: UIView {
    private let photo = UIImageView()
    private let scrim = UIView()
    override init(frame: CGRect) {
        super.init(frame: frame)
        isUserInteractionEnabled = false; clipsToBounds = true
        photo.contentMode = .scaleAspectFill; photo.clipsToBounds = true
        for child in [photo, scrim] {
            child.translatesAutoresizingMaskIntoConstraints = false; addSubview(child)
            NSLayoutConstraint.activate([
                child.leadingAnchor.constraint(equalTo: leadingAnchor), child.trailingAnchor.constraint(equalTo: trailingAnchor),
                child.topAnchor.constraint(equalTo: topAnchor), child.bottomAnchor.constraint(equalTo: bottomAnchor)
            ])
        }
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    func apply(_ theme: KeyboardTheme, image: UIImage?) {
        backgroundColor = theme.background; photo.image = image
        scrim.backgroundColor = theme.isDark(traitCollection) ? UIColor.black.withAlphaComponent(0.25) : UIColor.white.withAlphaComponent(0.20)
        scrim.isHidden = image == nil
    }
}
