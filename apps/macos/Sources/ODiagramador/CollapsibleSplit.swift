import AppKit
import SwiftUI

/// Two panes with a bar between them: drag the bar to resize, double-click it (or click its chevron) to
/// collapse or restore one pane — the code next to the preview, or the bottom panel.
struct CollapsibleSplit<First: View, Second: View>: View {
    enum Collapses { case first, second }

    let axis: Axis
    let collapses: Collapses
    @Binding var size: Double
    @Binding var collapsed: Bool
    var minSize: Double = 120
    var minOther: Double = 200
    var help: String = ""
    @ViewBuilder var first: () -> First
    @ViewBuilder var second: () -> Second

    @State private var dragStart: Double?

    var body: some View {
        GeometryReader { geo in
            let total = axis == .horizontal ? geo.size.width : geo.size.height
            let pane = collapsed ? 0 : min(max(size, minSize), max(minSize, total - minOther - 9))
            if axis == .horizontal {
                HStack(spacing: 0) {
                    first().frame(width: collapses == .first ? pane : nil).clipped()
                    bar(total: total, pane: pane)
                    second().frame(width: collapses == .second ? pane : nil).clipped()
                }
            } else {
                VStack(spacing: 0) {
                    first().frame(height: collapses == .first ? pane : nil).clipped()
                    bar(total: total, pane: pane)
                    second().frame(height: collapses == .second ? pane : nil).clipped()
                }
            }
        }
    }

    /// Which way the chevron points: towards where the pane goes (collapse) or comes back from (restore).
    private var chevron: String {
        switch (axis, collapses, collapsed) {
        case (.horizontal, .first, false), (.horizontal, .second, true): return "chevron.compact.left"
        case (.horizontal, _, _): return "chevron.compact.right"
        case (.vertical, .second, false), (.vertical, .first, true): return "chevron.compact.down"
        case (.vertical, _, _): return "chevron.compact.up"
        }
    }

    private func toggle() {
        withAnimation(.easeInOut(duration: 0.18)) { collapsed.toggle() }
    }

    private func bar(total: Double, pane: Double) -> some View {
        ZStack {
            Rectangle().fill(Color(nsColor: .separatorColor)).frame(width: axis == .horizontal ? 1 : nil, height: axis == .vertical ? 1 : nil)
            Image(systemName: chevron)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.secondary)
                .frame(width: axis == .horizontal ? 9 : 28, height: axis == .horizontal ? 28 : 9)
                .background(Color(nsColor: .windowBackgroundColor), in: RoundedRectangle(cornerRadius: 3))
                .overlay(RoundedRectangle(cornerRadius: 3).stroke(Color(nsColor: .separatorColor)))
                .onTapGesture(perform: toggle)
        }
        .frame(width: axis == .horizontal ? 9 : nil, height: axis == .vertical ? 9 : nil)
        .contentShape(Rectangle())
        .help(help)
        .onHover { inside in
            if inside { (axis == .horizontal ? NSCursor.resizeLeftRight : NSCursor.resizeUpDown).push() } else { NSCursor.pop() }
        }
        .gesture(
            DragGesture(minimumDistance: 2)
                .onChanged { g in
                    let start = dragStart ?? (collapsed ? 0 : pane)
                    dragStart = start
                    let delta = axis == .horizontal ? g.translation.width : g.translation.height
                    let next = collapses == .first ? start + delta : start - delta
                    if collapsed, next > 24 { collapsed = false }
                    size = min(max(next, minSize), total - minOther - 9)
                }
                .onEnded { _ in dragStart = nil }
        )
        .simultaneousGesture(TapGesture(count: 2).onEnded(toggle))
    }
}
