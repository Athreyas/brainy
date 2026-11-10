import Foundation

// TaskService — shells out to `task export`, decodes JSON, buckets tasks,
// completes and adds tasks via the CLI.

final class TaskService: ObservableObject {
    static let shared = TaskService()

    @Published var all: [TaskItem] = []
    @Published var pending: [TaskItem] = []
    @Published var isLoading = false
    @Published var error: String?

    var overdue: [TaskItem] { pending.filter(\.isOverdue) }
    var dueThisWeek: [TaskItem] {
        let week = Date().addingTimeInterval(7 * 24 * 3600)
        return pending.filter { t in
            guard let d = t.dueDate else { return false }
            return d >= Date() && d <= week
        }
    }
    var highPriority: [TaskItem] { pending.filter { $0.priorityRank == 0 } }
    var later: [TaskItem] { pending.filter { !$0.isOverdue && !dueThisWeek.contains($0) && $0.priorityRank != 0 } }

    func reload() {
        isLoading = true
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let process = Process()
                process.executableURL = URL(fileURLWithPath: "/opt/homebrew/bin/task")
                process.arguments = ["export"]
                let pipe = Pipe()
                process.standardOutput = pipe
                process.standardError = pipe
                try process.run()
                process.waitUntilExit()
                let data = pipe.fileHandleForReading.readDataToEndOfFile()
                guard process.terminationStatus == 0 else {
                    throw NSError(domain: "TaskService", code: 1,
                                    userInfo: [NSLocalizedDescriptionKey: "task export failed"])
                }
                var tasks = try JSONDecoder().decode([TaskItem].self, from: data)
                tasks.sort {
                    let a = $0.due ?? "9999"
                    let b = $1.due ?? "9999"
                    if a != b { return a < b }
                    return $0.urgency > $1.urgency
                }
                DispatchQueue.main.async {
                    self.all = tasks
                    self.pending = tasks.filter { $0.status == "pending" }
                    self.isLoading = false
                    self.error = nil
                }
            } catch {
                DispatchQueue.main.async {
                    self.error = error.localizedDescription
                    self.isLoading = false
                }
            }
        }
    }

    func complete(_ task: TaskItem) {
        DispatchQueue.global(qos: .userInitiated).async {
            let p = Process()
            p.executableURL = URL(fileURLWithPath: "/opt/homebrew/bin/task")
            p.arguments = [String(task.id), "done"]
            p.standardOutput = Pipe()
            p.standardError = Pipe()
            try? p.run()
            p.waitUntilExit()
            DispatchQueue.main.async { self.reload() }
        }
    }

    func add(description: String, project: String = "", priority: String = "") {
        var args = [description]
        if !project.isEmpty { args.append("project:\(project)") }
        if !priority.isEmpty { args.append("priority:\(priority)") }
        args.append("+vault")
        DispatchQueue.global(qos: .userInitiated).async {
            let p = Process()
            p.executableURL = URL(fileURLWithPath: "/opt/homebrew/bin/task")
            p.arguments = args
            p.standardOutput = Pipe()
            p.standardError = Pipe()
            try? p.run()
            p.waitUntilExit()
            DispatchQueue.main.async { self.reload() }
        }
    }

    func projectGroups() -> [ProjectGroup] {
        let grouped = Dictionary(grouping: pending) { $0.project.isEmpty ? "Inbox" : $0.project }
        return grouped.values.map { items in
            ProjectGroup(name: items.first?.project.isEmpty == false ? items.first!.project : "Inbox",
                         tasks: items.sorted { $0.urgency > $1.urgency })
        }.sorted { $0.name < $1.name }
    }
}
