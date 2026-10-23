import SwiftUI

/// Markdown drawn with SwiftUI text (blocks come from the core's `markdown_blocks`, the same on every platform).
struct MarkdownView: View {
    let blocks: [MDBlock]

    init(_ markdown: String) {
        blocks = (try? Core.call("markdown_blocks", ["text": markdown], as: [MDBlock].self)) ?? [MDBlock(kind: "paragraph", level: 0, number: nil, spans: [MDSpan(text: markdown)])]
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ForEach(Array(blocks.enumerated()), id: \.offset) { _, block in
                row(block)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .textSelection(.enabled)
    }

    @ViewBuilder private func row(_ b: MDBlock) -> some View {
        switch b.kind {
        case "heading":
            Text(b.attributed(base: b.level <= 2 ? .headline : .subheadline.weight(.semibold))).padding(.top, 6)
        case "bullet":
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Text(b.number.map { "\($0)." } ?? (b.level > 1 ? "◦" : "•")).foregroundStyle(.secondary)
                Text(b.attributed).fixedSize(horizontal: false, vertical: true)
            }
            .padding(.leading, CGFloat(max(0, b.level - 1)) * 16)
        case "code":
            Text(b.attributed(base: .system(.callout, design: .monospaced)))
                .padding(8).frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.secondary.opacity(0.12), in: RoundedRectangle(cornerRadius: 6))
        case "quote":
            Text(b.attributed).italic().foregroundStyle(.secondary).padding(.leading, 10)
                .overlay(alignment: .leading) { Rectangle().fill(.secondary.opacity(0.4)).frame(width: 3) }
        case "rule":
            Divider()
        default:
            Text(b.attributed).fixedSize(horizontal: false, vertical: true)
        }
    }
}

struct MDSpan: Codable, Hashable {
    var text: String
    var bold: Bool? = nil
    var italic: Bool? = nil
    var code: Bool? = nil
    var link: String? = nil
}

struct MDBlock: Codable, Hashable {
    var kind: String
    var level: Int
    var number: Int?
    var spans: [MDSpan]

    var attributed: AttributedString { attributed(base: .callout) }

    /// Spans carry their own font (bold, code…), so the block's font goes in here, not on the Text.
    func attributed(base: Font) -> AttributedString {
        var out = AttributedString()
        for s in spans {
            var part = AttributedString(s.text)
            var font: Font = base
            if s.code == true { font = .system(.callout, design: .monospaced) }
            if s.bold == true { font = font.bold() }
            if s.italic == true { font = font.italic() }
            part.font = font
            if let link = s.link, let url = URL(string: link) { part.link = url }
            out += part
        }
        return out
    }
}
