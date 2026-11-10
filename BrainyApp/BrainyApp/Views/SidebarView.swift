import SwiftUI

struct SidebarView: View {
    @EnvironmentObject var app: AppState

    var body: some View {
        List(selection: $app.selection) {
            Section("Brain") {
                Label("Memory", systemImage: "brain.head.profile")
                    .tag(AppState.Selection.memory)
                Label("Actions", systemImage: "checklist")
                    .tag(AppState.Selection.actions)
                Label("Calendar", systemImage: "calendar")
                    .tag(AppState.Selection.calendar)
                Label("Search", systemImage: "magnifyingglass")
                    .tag(AppState.Selection.search)
            }
            Section("Work") {
                Label("Tasks", systemImage: "checklist.checked")
                    .tag(AppState.Selection.tasks)
                Label("Bookmarks", systemImage: "bookmark")
                    .tag(AppState.Selection.bookmarks)
            }
            Section("Projects") {
                ForEach(app.tasks.projectGroups()) { g in
                    Label("\(g.name) (\(g.tasks.count))", systemImage: "folder")
                        .tag(AppState.Selection.taskProject(g.name))
                }
            }
            Section("Bookmarks") {
                ForEach(app.bookmarks.categories) { c in
                    Label("\(c.name) (\(c.bookmarks.count))", systemImage: "bookmark.fill")
                        .tag(AppState.Selection.bookmarkCategory(c.name))
                }
            }
        }
        .listStyle(.sidebar)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { app.reloadAll() } label: { Label("Refresh", systemImage: "arrow.clockwise") }
            }
        }
    }
}
