import SwiftUI

@main
struct BrainyApp: App {
    @StateObject private var app = AppState()

    var body: some Scene {
        WindowGroup {
            NavigationSplitView {
                SidebarView().environmentObject(app)
            } detail: {
                DetailView().environmentObject(app)
            }
            .onAppear { app.reloadAll() }
        }
        .windowToolbarStyle(.unified)
    }
}

struct DetailView: View {
    @EnvironmentObject var app: AppState

    var body: some View {
        switch app.selection {
        case .memory: MemoryView()
        case .actions: ActionsView()
        case .calendar: CalendarView()
        case .search: SearchView()
        case .tasks: TasksView()
        case .bookmarks: BookmarksView()
        case .note(let note): NoteDetailView(note: note)
        case .taskProject(let name): ProjectTasksView(name: name)
        case .bookmarkCategory(let name): BookmarkCategoryView(name: name)
        case .none: MemoryView()
        }
    }
}

struct NoteDetailView: View {
    var note: Note
    var body: some View {
        ScrollView { Text(note.body).padding() }
            .navigationTitle(note.title)
    }
}

struct ProjectTasksView: View {
    var name: String
    @EnvironmentObject var app: AppState
    var body: some View {
        List(app.tasks.projectGroups().first(where: { $0.name == name })?.tasks ?? []) { t in
            TaskRow(task: t)
        }
        .navigationTitle(name)
    }
}

struct BookmarkCategoryView: View {
    var name: String
    @EnvironmentObject var app: AppState
    var body: some View {
        List(app.bookmarks.categories.first(where: { $0.name == name })?.bookmarks ?? []) { b in
            VStack(alignment: .leading) {
                Text(b.name).font(.body)
                Text(b.url).font(.caption).foregroundStyle(.secondary)
                    .lineLimit(1).truncationMode(.middle)
            }
            .onTapGesture { if let url = URL(string: b.url) { NSWorkspace.shared.open(url) } }
        }
        .navigationTitle(name)
    }
}
