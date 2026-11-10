import SwiftUI

struct TasksView: View {
    @EnvironmentObject var app: AppState
    @State private var newDescription = ""

    var body: some View {
        List {
            Section(header: Text(TaskBucket.overdue.rawValue + " (\(app.tasks.overdue.count))")) {
                ForEach(app.tasks.overdue) { t in TaskRow(task: t) }
            }
            Section(header: Text(TaskBucket.thisWeek.rawValue + " (\(app.tasks.dueThisWeek.count))")) {
                ForEach(app.tasks.dueThisWeek) { t in TaskRow(task: t) }
            }
            Section(header: Text(TaskBucket.highPrio.rawValue + " (\(app.tasks.highPriority.count))")) {
                ForEach(app.tasks.highPriority) { t in TaskRow(task: t) }
            }
            Section(header: Text(TaskBucket.later.rawValue + " (\(app.tasks.later.count))")) {
                ForEach(app.tasks.later) { t in TaskRow(task: t) }
            }
            Section("Add task") {
                HStack {
                    TextField("New task…", text: $newDescription)
                        .textFieldStyle(.roundedBorder)
                    Button("Add") {
                        guard !newDescription.isEmpty else { return }
                        app.tasks.add(description: newDescription)
                        newDescription = ""
                    }
                }
            }
        }
        .navigationTitle("Tasks (\(app.tasks.pending.count) open)")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Text("\(app.tasks.pending.count) pending · \(app.tasks.overdue.count) overdue")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}

struct TaskRow: View {
    var task: TaskItem
    @EnvironmentObject var app: AppState

    var body: some View {
        HStack(alignment: .top) {
            Button { app.tasks.complete(task) } label: {
                Image(systemName: "circle")
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            VStack(alignment: .leading) {
                Text(task.description).font(.body)
                HStack(spacing: 8) {
                    if !task.project.isEmpty {
                        Text(task.project).font(.caption).padding(.horizontal, 6).padding(.vertical, 2)
                            .background(Color.gray.opacity(0.15), in: Capsule())
                    }
                    if let d = task.dueDate {
                        Text(d.formatted(date: .abbreviated, time: .omitted))
                            .font(.caption).foregroundStyle(task.isOverdue ? .red : .secondary)
                    }
                    Text("urg \(String(format: "%.1f", task.urgency))").font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .padding(.vertical, 2)
        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
            Button("Done") { app.tasks.complete(task) }
            Button("Delete", role: .destructive) { /* via task delete */ }
        }
        .swipeActions(edge: .leading) {
            Button("Later") { app.tasks.modificationPlaceholder(task) }
        }
    }
}

extension TaskService {
    func modificationPlaceholder(_ task: TaskItem) {}
}
