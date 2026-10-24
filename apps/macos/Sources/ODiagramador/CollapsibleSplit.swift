import AppKit
import SwiftUI

/// Two panes with a bar between them: drag the bar to resize, double-click it (or click its chevron) to
/// collapse or restore one pane — the code next to the preview, or the bottom panel. A collapsed pane
/// leaves a labelled strip ("CODE ›") so it is clear what is hidden and where to click to bring it back.
struct CollapsibleSplit<First: View, Second: View>: View {
    enum Collapses { case first, second }

    let axis: Axis
    let collapses: Collapses
    @Binding var size: Double
    @Binding var collapsed: Bool
    var minSize: Double = 120
    var minOther: Double = 200
    var help: String = ""
    /// Shown on the strip a collapsed pane leaves behind, and its tooltip.
    var collapsedTitle: String = ""
    var collapsedIcon: String = "sidebar.left"
    var collapsedHelp: String = ""
    @ViewBuilder var first: () -> First
    @ViewBuilder var second: () -> Second

    @State private var dragStart: Double?

    var body: some View {
        GeometryReader { geo in
            let total = axis == .horizontal ? geo.size.width : geo.size.height
            let pane = collapsed ? 0 : min(max(size, minSize), max(minSize, total - minOther - 9))
            // (a collapsed pane is replaced by its labelled strip, see collapsedRail)
            if axis == .horizontal {
                HStack(spacing: 0) {
                    first().frame(width: collapses == .first ? pane : nil).clipped().modifier(Hidden(on: collapsed && collapses == .first))
                    bar(total: total, pane: pane)
                    second().frame(width: collapses == .second ? pane : nil).clipped().modifier(Hidden(on: collapsed && collapses == .second))
                }
            } else {
                VStack(spacing: 0) {
                    first().frame(height: collapses == .first ? pane : nil).clipped().modifier(Hidden(on: collapsed && collapses == .first))
                    bar(total: total, pane: pane)
                    second().frame(height: collapses == .second ? pane : nil).clipped().modifier(Hidden(on: collapsed && collapses == .second))
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

    /// Thickness of the strip a collapsed pane leaves.
    private let rail: Double = 24

    @ViewBuilder
    private func bar(total: Double, pane: Double) -> some View {
        if collapsed { collapsedRail(total: total) } else { divider(total: total, pane: pane) }
    }

    /// The hidden pane's name and an arrow: click (or drag) it to bring the pane back.
    private func collapsedRail(total: Double) -> some View {
        let title = Text(collapsedTitle).textCase(.uppercase).lineLimit(1).fixedSize()
        return Group {
            if axis == .horizontal {
                // arrow and icon upright, the name written top to bottom below them
                VStack(spacing: 8) {
                    Image(systemName: chevron)
                    Image(systemName: collapsedIcon)
                    title.frame(width: 240, height: rail, alignment: .leading).rotationEffect(.degrees(90)).frame(width: rail, height: 240)
                    Spacer(minLength: 0)
                }
                .padding(.top, 10)
            } else {
                HStack(spacing: 6) {
                    Image(systemName: chevron)
                    Image(systemName: collapsedIcon)
                    title
                    Spacer(minLength: 0)
                }
                .padding(.leading, 10)
            }
        }
        .font(.caption.weight(.semibold))
        .foregroundStyle(.secondary)
        .frame(width: axis == .horizontal ? rail : nil, height: axis == .vertical ? rail : nil)
        .background(.bar)
        .overlay(alignment: axis == .horizontal ? .leading : .top) {
            Rectangle().fill(Color(nsColor: .separatorColor)).frame(width: axis == .horizontal ? 1 : nil, height: axis == .vertical ? 1 : nil)
        }
        .overlay(alignment: axis == .horizontal ? .trailing : .bottom) {
            Rectangle().fill(Color(nsColor: .separatorColor)).frame(width: axis == .horizontal ? 1 : nil, height: axis == .vertical ? 1 : nil)
        }
        .contentShape(Rectangle())
        .help(collapsedHelp.isEmpty ? help : collapsedHelp)
        .onHover { inside in
            if inside { NSCursor.pointingHand.push() } else { NSCursor.pop() }
        }
        .onTapGesture(perform: toggle)
        .gesture(resize(total: total, pane: 0))
    }

    private func resize(total: Double, pane: Double) -> some Gesture {
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
    }

    private func divider(total: Double, pane: Double) -> some View {
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
        .gesture(resize(total: total, pane: pane))
        .simultaneousGesture(TapGesture(count: 2).onEnded(toggle))
    }
}

/// A collapsed pane stays alive (keeps its state) but is invisible to the mouse and to accessibility,
/// so clicks on the strip it leaves are not caught by its zero-height content.
private struct Hidden: ViewModifier {
    let on: Bool
    func body(content: Content) -> some View {
        content.opacity(on ? 0 : 1).allowsHitTesting(!on).accessibilityHidden(on)
    }
}
