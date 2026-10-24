import AppKit
import SwiftUI

/// Plain-text Mermaid editor: monospace NSTextView with a line-number gutter and error lines tinted.
struct SourceEditor: NSViewRepresentable {
    @Binding var text: String
    var errorLines: Set<Int>
    var warningLines: Set<Int>
    @Binding var scrollToLine: Int?
    var font: NSFont = .monospacedSystemFont(ofSize: 13, weight: .regular)
    /// False in view mode: the source can be read and selected, not typed into.
    var editable = true

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        // TextKit 1 on purpose: the gutter and line tinting use NSLayoutManager
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        scroll.borderType = .noBorder
        let tv = NSTextView(usingTextLayoutManager: false)
        tv.minSize = .zero
        tv.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        tv.isVerticallyResizable = true
        tv.isHorizontallyResizable = false
        tv.autoresizingMask = [.width]
        tv.textContainer?.widthTracksTextView = true
        tv.textContainer?.containerSize = NSSize(width: 0, height: CGFloat.greatestFiniteMagnitude)
        scroll.documentView = tv
        tv.isRichText = false
        tv.font = .monospacedSystemFont(ofSize: 13, weight: .regular)
        tv.textColor = .textColor
        tv.backgroundColor = .textBackgroundColor
        tv.drawsBackground = true
        tv.typingAttributes = [.font: NSFont.monospacedSystemFont(ofSize: 13, weight: .regular), .foregroundColor: NSColor.textColor]
        tv.isAutomaticQuoteSubstitutionEnabled = false
        tv.isAutomaticDashSubstitutionEnabled = false
        tv.isAutomaticTextReplacementEnabled = false
        tv.isAutomaticSpellingCorrectionEnabled = false
        tv.isContinuousSpellCheckingEnabled = false
        tv.allowsUndo = true
        tv.textContainerInset = NSSize(width: 4, height: 8)
        tv.delegate = context.coordinator
        tv.string = text
        let ruler = LineNumberRuler(textView: tv)
        ruler.clientView = tv
        scroll.verticalRulerView = ruler
        scroll.hasVerticalRuler = true
        scroll.rulersVisible = true
        context.coordinator.textView = tv
        context.coordinator.ruler = ruler
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let tv = context.coordinator.textView else { return }
        if tv.isEditable != editable { tv.isEditable = editable }
        if tv.font != font {
            tv.font = font
            tv.typingAttributes[.font] = font
            context.coordinator.ruler?.needsDisplay = true
        }
        if tv.string != text {
            let sel = tv.selectedRanges
            tv.string = text
            // AppKit throws on an empty selection list: when the text shrank (element deleted,
            // agent rewrite) keep the ranges that still fit, else put the caret at the end.
            let length = (text as NSString).length
            let kept = sel.filter { $0.rangeValue.upperBound <= length }
            tv.selectedRanges = kept.isEmpty ? [NSValue(range: NSRange(location: length, length: 0))] : kept
        }
        context.coordinator.highlight()
        if let line = scrollToLine {
            context.coordinator.reveal(line: line)
            DispatchQueue.main.async { scrollToLine = nil }
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: SourceEditor
        weak var textView: NSTextView?
        weak var ruler: LineNumberRuler?

        init(_ parent: SourceEditor) { self.parent = parent }

        func textDidChange(_ notification: Notification) {
            guard let tv = textView else { return }
            parent.text = tv.string
            ruler?.needsDisplay = true
        }

        private func range(ofLine line: Int, in s: NSString) -> NSRange? {
            var current = 1
            var found: NSRange?
            s.enumerateSubstrings(in: NSRange(location: 0, length: s.length), options: [.byLines, .substringNotRequired]) { _, range, enclosing, stop in
                if current == line {
                    found = enclosing.length > 0 ? enclosing : range
                    stop.pointee = true
                }
                current += 1
            }
            return found
        }

        func highlight() {
            guard let tv = textView, let lm = tv.layoutManager else { return }
            let s = tv.string as NSString
            lm.removeTemporaryAttribute(.backgroundColor, forCharacterRange: NSRange(location: 0, length: s.length))
            for (lines, color) in [(parent.warningLines, NSColor.systemOrange.withAlphaComponent(0.15)),
                                   (parent.errorLines, NSColor.systemRed.withAlphaComponent(0.22))] {
                for line in lines {
                    if let r = range(ofLine: line, in: s) {
                        lm.addTemporaryAttribute(.backgroundColor, value: color, forCharacterRange: r)
                    }
                }
            }
            ruler?.errorLines = parent.errorLines
            ruler?.needsDisplay = true
        }

        func reveal(line: Int) {
            guard let tv = textView, let r = range(ofLine: line, in: tv.string as NSString) else { return }
            tv.scrollRangeToVisible(r)
            tv.setSelectedRange(NSRange(location: r.location, length: 0))
            tv.showFindIndicator(for: r)
        }
    }
}

/// Gutter with line numbers; lines with errors are drawn in red.
final class LineNumberRuler: NSRulerView {
    weak var textView: NSTextView?
    var errorLines: Set<Int> = []

    init(textView: NSTextView) {
        self.textView = textView
        super.init(scrollView: textView.enclosingScrollView, orientation: .verticalRuler)
        ruleThickness = 40
        clipsToBounds = true // macOS 14+ no longer clips by default; the gutter would paint over the text
        NotificationCenter.default.addObserver(self, selector: #selector(redraw), name: NSView.boundsDidChangeNotification, object: textView.enclosingScrollView?.contentView)
    }

    required init(coder: NSCoder) { fatalError("init(coder:) is not used") }

    @objc private func redraw() { needsDisplay = true }

    override func drawHashMarksAndLabels(in rect: NSRect) {
        guard let tv = textView, let lm = tv.layoutManager, let tc = tv.textContainer else { return }
        NSColor.textBackgroundColor.setFill()
        rect.intersection(bounds).fill()
        let s = tv.string as NSString
        let visible = lm.glyphRange(forBoundingRect: tv.visibleRect, in: tc)
        let firstChar = lm.characterIndexForGlyph(at: visible.location)
        var line = 1
        s.enumerateSubstrings(in: NSRange(location: 0, length: firstChar), options: [.byLines, .substringNotRequired]) { _, _, _, _ in line += 1 }
        let font = NSFont.monospacedDigitSystemFont(ofSize: 11, weight: .regular)
        var index = firstChar
        let origin = convert(NSPoint.zero, from: tv)
        repeat {
            let lineRange = s.lineRange(for: NSRange(location: index, length: 0))
            let glyphs = lm.glyphRange(forCharacterRange: lineRange, actualCharacterRange: nil)
            var lineRect = lm.lineFragmentRect(forGlyphAt: min(glyphs.location, max(lm.numberOfGlyphs - 1, 0)), effectiveRange: nil)
            if s.length == 0 { lineRect = NSRect(x: 0, y: 0, width: 0, height: 16) }
            let y = lineRect.minY + origin.y + tv.textContainerInset.height
            let color: NSColor = errorLines.contains(line) ? .systemRed : .secondaryLabelColor
            let label = "\(line)" as NSString
            let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
            let size = label.size(withAttributes: attrs)
            label.draw(at: NSPoint(x: ruleThickness - size.width - 6, y: y + (lineRect.height - size.height) / 2), withAttributes: attrs)
            if y > rect.maxY { break }
            index = NSMaxRange(lineRange)
            line += 1
        } while index < s.length
    }
}
