import Foundation

// VaultService — walks the Obsidian vault, parses markdown + frontmatter,
// groups notes by folder, extracts open checkboxes, daily notes, calendar data.

final class VaultService: ObservableObject {
    static let shared = VaultService()

    @Published var snapshot = VaultSnapshot()
    @Published var openActions: [ActionItem] = []
    @Published var isLoading = false
    @Published var error: String?

    let vaultRoot: URL = {
        if let p = ProcessInfo.processInfo.environment["BRAINY_VAULT"] {
            return URL(fileURLWithPath: p)
        }
        return URL(fileURLWithPath: "/Users/art/Library/Mobile Documents/iCloud~md~obsidian/Documents/home-vault")
    }()

    func reload() {
        isLoading = true
        let root = vaultRoot
        DispatchQueue.global(qos: .userInitiated).async {
            let snap = Self.loadSynchronously(root: root)
            DispatchQueue.main.async {
                self.snapshot = snap
                self.openActions = Self.extractActions(snap)
                self.isLoading = false
            }
        }
    }

    private static func loadSynchronously(root: URL) -> VaultSnapshot {
        var snap = VaultSnapshot()
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(at: root, includingPropertiesForKeys: [.isRegularFileKey], options: [.skipsHiddenFiles]) else {
            return snap
        }
        let rootPath = root.path
        for case let url as URL in enumerator {
            guard url.pathExtension == "md" else { continue }
            if url.pathComponents.contains(".obsidian") || url.pathComponents.contains(".git") { continue }
            guard let note = parseNote(at: url, vaultRootPath: rootPath) else { continue }
            snap.notes.append(note)
        }
        snap.notes.sort { $0.path < $1.path }
        snap.folders = Array(Set(snap.notes.map(\.folder))).sorted()
        snap.dailyNotes = snap.notes.filter(\.isDaily).sorted { $0.path > $1.path }
        snap.weeklyNotes = snap.notes.filter(\.isWeekly).sorted { $0.path > $1.path }
        snap.memoryNotes = snap.notes.filter(\.isMemory)
        snap.sessionNotes = snap.notes.filter(\.isSession)
        snap.factNotes = snap.notes.filter(\.isFact)
        return snap
    }

    private static func parseNote(at url: URL, vaultRootPath: String) -> Note? {
        guard let content = try? String(contentsOf: url, encoding: .utf8) else { return nil }
        let rel = url.path.replacingOccurrences(of: vaultRootPath + "/", with: "")
        let folder = (rel as NSString).deletingLastPathComponent
        let (meta, body) = splitFrontmatter(content)
        let stem = url.deletingPathExtension().lastPathComponent
        let title = meta["title"] ?? (stem.isEmpty ? rel : stem)
        let tags = (meta["tags"] ?? "").split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
        let author = meta["author"] ?? ""
        let type = meta["type"] ?? ""
        let updated = (meta["updated"] ?? meta["created"] ?? "").toDate()
        return Note(path: rel, title: title, folder: folder.isEmpty ? "(root)" : folder,
                    type: type, tags: tags, updated: updated, author: author, body: body)
    }

    private static func splitFrontmatter(_ content: String) -> ([String: String], String) {
        guard content.hasPrefix("---") else { return ([:], content) }
        let parts = content.components(separatedBy: "---")
        guard parts.count >= 3 else { return ([:], content) }
        let yaml = parts[1]
        let body = parts.dropFirst(2).joined(separator: "---")
        var meta: [String: String] = [:]
        for line in yaml.components(separatedBy: "\n") {
            guard let i = line.firstIndex(of: ":") else { continue }
            let k = String(line[..<i]).trimmingCharacters(in: .whitespaces)
            let v = String(line[line.index(after: i)...]).trimmingCharacters(in: .whitespaces)
                .trimmingCharacters(in: CharacterSet(charactersIn: "\"'"))
            meta[k] = v
        }
        return (meta, body)
    }

    static func extractActions(_ snap: VaultSnapshot) -> [ActionItem] {
        var out: [ActionItem] = []
        for n in snap.notes {
            if n.path.hasPrefix("00-Inbox") || n.path.hasPrefix("06-Memory") || n.path.hasPrefix("08-Archive") { continue }
            for line in n.body.components(separatedBy: "\n") {
                let t = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if t.hasPrefix("- [ ]") {
                    let text = t.dropFirst(5).trimmingCharacters(in: .whitespaces)
                    out.append(ActionItem(text: text, notePath: n.path, noteTitle: n.title))
                }
            }
        }
        return out
    }

    // Calendar activity: (date, title) for each daily note, parsed from filename
    func calendarActivity() -> [(Date, String)] {
        let fmt = DateFormatter()
        fmt.dateFormat = "yyyy-MM-dd"
        fmt.locale = Locale(identifier: "en_US_POSIX")
        return snapshot.dailyNotes.compactMap { n -> (Date, String)? in
            let stem = (n.path as NSString).lastPathComponent.replacingOccurrences(of: ".md", with: "")
            guard let d = fmt.date(from: stem) else { return nil }
            return (d, n.title)
        }.sorted { $0.0 < $1.0 }
    }

    func search(_ query: String) -> [Note] {
        let q = query.lowercased()
        return snapshot.notes.filter {
            $0.title.lowercased().contains(q)
            || $0.path.lowercased().contains(q)
            || $0.body.lowercased().contains(q)
            || $0.tags.contains(where: { $0.lowercased().contains(q) })
        }
    }
}

extension String {
    func toDate() -> Date? {
        let fmt = DateFormatter()
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.dateFormat = "yyyy-MM-dd"
        return fmt.date(from: self)
    }
}
