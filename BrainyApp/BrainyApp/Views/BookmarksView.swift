import SwiftUI

struct BookmarksView: View {
    @EnvironmentObject var app: AppState

    var body: some View {
        List {
            ForEach(app.bookmarks.categories) { cat in
                Section(header: Text("\(cat.name) (\(cat.bookmarks.count))")) {
                    ForEach(cat.bookmarks) { b in
                        VStack(alignment: .leading) {
                            Text(b.name).font(.body)
                            HStack {
                                Text(b.host).font(.caption).foregroundStyle(.secondary)
                                Text("· " + b.browser).font(.caption).foregroundStyle(.secondary)
                                Text("· " + b.folder).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        .contentShape(Rectangle())
                        .onTapGesture {
                            if let url = URL(string: b.url) { NSWorkspace.shared.open(url) }
                        }
                    }
                }
            }
        }
        .navigationTitle("Bookmarks (\(app.bookmarks.bookmarks.count))")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Text(app.bookmarks.browserCounts.map { "\($0.0): \($0.1)" }.joined(separator: "  "))
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}
