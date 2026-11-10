import SwiftUI

struct ActionsView: View {
    @EnvironmentObject var app: AppState

    var body: some View {
        List(app.vault.openActions) { a in
            HStack(alignment: .top) {
                Image(systemName: "square")
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading) {
                    Text(a.text).font(.body)
                    Text("\(a.noteTitle) → \(a.notePath)").font(.caption).foregroundStyle(.secondary)
                }
            }
            .padding(.vertical, 2)
            .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                Button("Open") { /* Taskwarrior sync will flip the note checkbox */ }
            }
        }
        .navigationTitle("Actions (\(app.vault.openActions.count))")
    }
}
