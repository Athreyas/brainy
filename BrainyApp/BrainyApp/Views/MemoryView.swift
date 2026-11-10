import SwiftUI

struct MemoryView: View {
    @EnvironmentObject var app: AppState

    var body: some View {
        List {
            Section(header: Text("Session Logs — \(app.vault.snapshot.sessionNotes.count)")) {
                ForEach(app.vault.snapshot.sessionNotes) { n in
                    VStack(alignment: .leading) {
                        Text(n.title).font(.headline)
                        Text(n.path).font(.caption).foregroundStyle(.secondary)
                    }
                }
            }
            Section(header: Text("Atomic Facts — \(app.vault.snapshot.factNotes.count)")) {
                ForEach(app.vault.snapshot.factNotes) { n in
                    VStack(alignment: .leading) {
                        Text(n.title).font(.callout)
                        HStack {
                            Text(n.folder).font(.caption).foregroundStyle(.secondary)
                            if !n.tags.isEmpty { Text(n.tags.joined(separator: " · ")).font(.caption).foregroundStyle(.secondary) }
                        }
                    }
                }
            }
            Section(header: Text("Folders")) {
                ForEach(app.vault.snapshot.folders, id: \.self) { f in
                    HStack {
                        Text(f)
                        Spacer()
                        Text("\(app.vault.snapshot.notes.filter { $0.folder == f }.count)").foregroundStyle(.secondary)
                    }
                }
            }
        }
        .navigationTitle("Memory")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Text("\(app.vault.snapshot.notes.count) notes total").font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}
