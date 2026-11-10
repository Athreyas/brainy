import SwiftUI

struct SearchView: View {
    @EnvironmentObject var app: AppState
    @State private var query = ""

    var results: [Note] {
        guard !query.isEmpty else { return [] }
        return app.vault.search(query)
    }

    var body: some View {
        VStack(spacing: 0) {
            TextField("Search all notes…", text: $query)
                .textFieldStyle(.roundedBorder)
                .padding()
            List(results) { n in
                VStack(alignment: .leading) {
                    Text(n.title).font(.headline)
                    Text(n.path).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .navigationTitle("Search")
    }
}
