import Foundation

// BookmarkService — reads Chromium `Bookmarks` JSON from every Dia / Chrome /
// Arc / Edge / Brave / Vivaldi profile on this Mac, flattens the tree, and
// buckets bookmarks into smart categories.

final class BookmarkService: ObservableObject {
    static let shared = BookmarkService()

    @Published var bookmarks: [Bookmark] = []
    @Published var categories: [BookmarkCategory] = []
    @Published var browserCounts: [(String, Int)] = []
    @Published var isLoading = false

    struct ChromiumRoot: Decodable {
        struct Node: Decodable {
            let type: String
            let name: String?
            let url: String?
            let children: [Node]?
        }
        let roots: [String: Node]
    }

    func reload() {
        isLoading = true
        DispatchQueue.global(qos: .userInitiated).async {
            let all = Self.load()
            let cats = Self.categorize(all)
            let counts = Dictionary(grouping: all, by: \.browser).map { ($0.key, $0.value.count) }
                .sorted { $0.0 < $1.0 }
            DispatchQueue.main.async {
                self.bookmarks = all
                self.categories = cats
                self.browserCounts = counts
                self.isLoading = false
            }
        }
    }

    private static func load() -> [Bookmark] {
        var all: [Bookmark] = []
        for (browser, path) in browserFiles() {
            guard let data = try? Data(contentsOf: path) else { continue }
            guard let parsed = try? JSONDecoder().decode(ChromiumRoot.self, from: data) else { continue }
            var flat: [Bookmark] = []
            for root in parsed.roots.values { flatten(root, folder: "", browser: browser, into: &flat) }
            all.append(contentsOf: flat)
        }
        var seen = Set<String>()
        all = all.filter { seen.insert($0.url).inserted }
        all.sort { $0.folder == $1.folder ? $0.name < $1.name : $0.folder < $1.folder }
        return all.map { b in
            var c = b
            c.category = categorizeHeuristic(b)
            return c
        }
    }

    private static func flatten(_ node: ChromiumRoot.Node, folder: String, browser: String, into out: inout [Bookmark]) {
        if node.type == "folder" {
            let f = (node.name ?? folder)
            let next = folder.isEmpty ? f : "\(folder)/\(f)"
            for c in node.children ?? [] { flatten(c, folder: next, browser: browser, into: &out) }
        } else if node.type == "url", let url = node.url {
            let host = hostOf(url)
            out.append(Bookmark(name: node.name ?? url, url: url, host: host,
                                folder: folder, browser: browser, category: ""))
        }
    }

    static func hostOf(_ url: String) -> String {
        var u = url
        u = u.replacingOccurrences(of: "https://", with: "").replacingOccurrences(of: "http://", with: "")
        let host = u.components(separatedBy: "/").first ?? u
        return host.replacingOccurrences(of: "www.", with: "").components(separatedBy: "?").first ?? host
    }

    static func categorizeHeuristic(_ b: Bookmark) -> String {
        let host = b.host.lowercased()
        let folder = b.folder.lowercased()
        let url = b.url.lowercased()
        func hostIs(_ d: String) -> Bool { host == d || host.hasSuffix("." + d) }
        func folderHas(_ k: String) -> Bool { folder.contains(k) }
        func urlHas(_ k: String) -> Bool { url.contains(k) }

        if folderHas("vault") || urlHas("monk97.me") || hostIs("monk97.me") { return "Vault & Docs" }
        if hostIs("lab.monk97.me") || urlHas("tailscale.com") || urlHas("synology")
            || urlHas("portainer") || urlHas("proxmox") || urlHas("cloudflare") { return "Home Lab / Infra" }
        if hostIs("google.com") || hostIs("firebase.google.com") || hostIs("cloud.google.com")
            || hostIs("gdg.community.dev") || hostIs("jules.google.com") || folderHas("firebase")
            || folderHas("genkit") || urlHas("vertex-ai") || urlHas("gemini") { return "Google / Firebase / AI" }
        if hostIs("github.com") || hostIs("gitlab.com") || hostIs("code.visualstudio.com")
            || hostIs("stackoverflow.com") || hostIs("dev.to") { return "Code / Dev" }
        if hostIs("edx.org") || hostIs("coursera.org") || hostIs("udemy.com") || hostIs("docs.python.org")
            || hostIs("developer.mozilla.org") || hostIs("wikipedia.org")
            || folderHas("gyaan") || folderHas("learning") { return "Learning" }
        if hostIs("youtube.com") || hostIs("youtu.be") || hostIs("vimeo.com") { return "Video" }
        if hostIs("gmail.com") || hostIs("mail.google.com") || hostIs("notion.site") || hostIs("notion.so")
            || hostIs("x.com") || hostIs("twitter.com") || hostIs("instagram.com")
            || hostIs("linkedin.com") || hostIs("reddit.com") { return "Social / Productivity" }
        return "Unsorted"
    }

    static func categorize(_ bookmarks: [Bookmark]) -> [BookmarkCategory] {
        var map: [String: [Bookmark]] = [:]
        for b in bookmarks { map[b.category, default: []].append(b) }
        return map.map { BookmarkCategory(name: $0.key, bookmarks: $0.value) }.sorted { $0.name < $1.name }
    }

    static func browserFiles() -> [(String, URL)] {
        let base = URL(fileURLWithPath: NSHomeDirectory()).appendingPathComponent("Library/Application Support")
        let candidates: [(String, String)] = [
            ("Dia", "Dia/User Data"),
            ("Arc", "Arc/User Data"),
            ("Chrome", "Google/Chrome"),
            ("Edge", "Microsoft Edge"),
            ("Brave", "BraveSoftware/Brave-Browser"),
            ("Vivaldi", "Vivaldi"),
        ]
        var out: [(String, URL)] = []
        for (name, rel) in candidates {
            let dir = base.appendingPathComponent(rel)
            var profiles: [URL] = []
            let def = dir.appendingPathComponent("Default/Bookmarks")
            if FileManager.default.fileExists(atPath: def.path) { profiles.append(dir.appendingPathComponent("Default")) }
            if let items = try? FileManager.default.contentsOfDirectory(atPath: dir.path) {
                for item in items where item.hasPrefix("Profile") {
                    let p = dir.appendingPathComponent(item)
                    if FileManager.default.fileExists(atPath: p.appendingPathComponent("Bookmarks").path) {
                        profiles.append(p)
                    }
                }
            }
            for p in profiles { out.append((name, p.appendingPathComponent("Bookmarks"))) }
        }
        return out
    }
}
