import Foundation

// Vault models — mirror the Rust vault layer: one Note per markdown file,
// with frontmatter flattened to fields. Only fields the UI uses are kept.

struct Note: Identifiable, Hashable {
    let id = UUID()
    var path: String          // relative path inside vault
    var title: String
    var folder: String
    var type: String
    var tags: [String]
    var updated: Date?
    var author: String
    var body: String
}

extension Note {
    var isDaily: Bool { path.hasPrefix("09-Planning/Daily/") }
    var isWeekly: Bool { path.hasPrefix("09-Planning/Weekly/") }
    var isMemory: Bool { path.hasPrefix("06-Memory/") }
    var isSession: Bool { path.contains("/sessions/") }
    var isFact: Bool { path.contains("/memory/") }
}

struct VaultSnapshot {
    var notes: [Note] = []
    var folders: [String] = []
    var dailyNotes: [Note] = []
    var weeklyNotes: [Note] = []
    var memoryNotes: [Note] = []
    var sessionNotes: [Note] = []
    var factNotes: [Note] = []

    var openActions: [ActionItem] { [] } // filled by VaultService
}

struct ActionItem: Identifiable, Hashable {
    let id = UUID()
    var text: String
    var notePath: String
    var noteTitle: String
}

// Taskwarrior models — decoded straight from `task export` JSON.

struct TaskItem: Identifiable, Hashable, Codable {
    var id: Int
    var description: String
    var status: String
    var project: String = ""
    var tags: [String] = []
    var due: String? = nil
    var urgency: Double = 0
    var priority: String? = nil
    var entry: String? = nil

    var dueDate: Date? {
        guard let due else { return nil }
        let fmt = DateFormatter()
        fmt.locale = Locale(identifier: "en_US_POSIX")
        fmt.dateFormat = "yyyyMMdd'T'HHmmss'Z'"
        fmt.timeZone = TimeZone(secondsFromGMT: 0)
        return fmt.date(from: due)
    }

    var priorityRank: Int {
        switch priority {
        case "H": return 0
        case "M": return 1
        case "L": return 2
        default: return 3
        }
    }

    var isOverdue: Bool {
        guard let d = dueDate else { return false }
        return d < Date() && status == "pending"
    }
}

enum TaskBucket: String, CaseIterable, Identifiable {
    case overdue = "Overdue"
    case thisWeek = "Due This Week"
    case highPrio = "High Priority"
    case later = "Later"

    var id: String { rawValue }
}

struct ProjectGroup: Identifiable {
    var id: String { name }
    var name: String
    var tasks: [TaskItem]
}

// Bookmarks models — decoded from Chromium `Bookmarks` JSON.

struct Bookmark: Identifiable, Hashable {
    let id = UUID()
    var name: String
    var url: String
    var host: String
    var folder: String
    var browser: String
    var category: String
}

struct BookmarkCategory: Identifiable {
    var id: String { name }
    var name: String
    var bookmarks: [Bookmark]
}
