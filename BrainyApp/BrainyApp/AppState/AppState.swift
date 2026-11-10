import SwiftUI

@MainActor
final class AppState: ObservableObject {
    enum Selection: Hashable {
        case memory
        case actions
        case calendar
        case search
        case tasks
        case bookmarks
        case note(Note)
        case taskProject(String)
        case bookmarkCategory(String)
    }

    @Published var selection: Selection? = .memory
    @Published var lastError: String?

    let vault = VaultService.shared
    let tasks = TaskService.shared
    let bookmarks = BookmarkService.shared

    func reloadAll() {
        vault.reload()
        tasks.reload()
        bookmarks.reload()
    }
}
