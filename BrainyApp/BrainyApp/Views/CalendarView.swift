import SwiftUI

struct CalendarView: View {
    @EnvironmentObject var app: AppState

    private let monthFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "LLLL yyyy"
        return f
    }()
    private let dayFormatter: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "EEE MMM d"
        return f
    }()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 8) {
                    Text(monthFormatter.string(from: Date()))
                        .font(.title2).bold()
                    MonthGrid(activity: app.vault.calendarActivity())
                    HStack {
                        Text("● daily note").font(.caption).foregroundStyle(.secondary)
                        Spacer()
                        Text("· no note").font(.caption).foregroundStyle(.secondary)
                    }
                }
                .padding()

                Section(header: Text("Daily notes").font(.headline).padding(.horizontal)) {
                    ForEach(app.vault.snapshot.dailyNotes) { n in
                        VStack(alignment: .leading) {
                            Text(n.title).font(.body)
                            Text(n.path).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
                .padding(.horizontal)

                Section(header: Text("Weekly plans").font(.headline).padding(.horizontal)) {
                    ForEach(app.vault.snapshot.weeklyNotes) { n in
                        Text(n.title).font(.body)
                    }
                }
                .padding(.horizontal)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .navigationTitle("Calendar")
    }
}

struct MonthGrid: View {
    var activity: [(Date, String)]
    private let cal = Calendar.current

    var body: some View {
        let now = Date()
        let range = cal.range(of: .day, in: .month, for: now)!
        let cols = Array(repeating: GridItem(.fixed(28), spacing: 4), count: 7)
        LazyVGrid(columns: cols, spacing: 4) {
            ForEach(1..<range.upperBound, id: \.self) { day in
                let date = cal.date(byAdding: .day, value: day - 1, to: cal.date(from: cal.dateComponents([.year, .month], from: now))!)!
                let has = activity.contains { cal.isDate($0.0, inSameDayAs: date) }
                Text("\(day)")
                    .font(.system(size: 11))
                    .frame(width: 28, height: 28)
                    .background(has ? Color.accentColor.opacity(0.25) : Color.clear, in: Circle())
                    .overlay(Circle().stroke(has ? Color.accentColor : .clear, lineWidth: 1))
            }
        }
    }
}
