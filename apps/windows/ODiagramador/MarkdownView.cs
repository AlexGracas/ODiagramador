using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;

namespace ODiagramador;

public record MdSpan(string Text, bool? Bold, bool? Italic, bool? Code, string? Link);

public record MdBlock(string Kind, int Level, long? Number, List<MdSpan> Spans);

/// <summary>Markdown drawn as a WPF FlowDocument (blocks come from the core's markdown_blocks, the same on every platform).</summary>
public static class MarkdownView
{
    public static FlowDocumentScrollViewer Create(string markdown, double width, double maxHeight) => new()
    {
        Document = Build(markdown),
        Width = width,
        MaxHeight = maxHeight,
        VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
        HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
        IsToolBarVisible = false,
        Margin = new Thickness(0, 8, 0, 10),
    };

    public static FlowDocument Build(string markdown)
    {
        var blocks = Core.CallOrDefault<List<MdBlock>>("markdown_blocks", new { text = markdown })
                     ?? new List<MdBlock> { new("paragraph", 0, null, new List<MdSpan> { new(markdown, null, null, null, null) }) };
        var doc = new FlowDocument
        {
            PagePadding = new Thickness(0, 0, 8, 0),
            FontFamily = new FontFamily("Segoe UI"),
            FontSize = 13,
            TextAlignment = TextAlignment.Left,
        };
        foreach (var b in blocks)
        {
            if (b.Kind == "rule")
            {
                doc.Blocks.Add(new BlockUIContainer(new Separator { Margin = new Thickness(0, 4, 0, 4) }));
                continue;
            }
            var p = new Paragraph { Margin = new Thickness(0, 2, 0, 4) };
            switch (b.Kind)
            {
                case "heading":
                    p.FontSize = b.Level <= 2 ? 16 : 14;
                    p.FontWeight = FontWeights.SemiBold;
                    p.Margin = new Thickness(0, 10, 0, 4);
                    break;
                case "bullet":
                    var indent = 18 * Math.Max(1, b.Level);
                    p.Margin = new Thickness(indent, 1, 0, 2);
                    p.TextIndent = -14;
                    p.Inlines.Add(new Run((b.Number is long n ? $"{n}." : b.Level > 1 ? "◦" : "•") + " ") { Foreground = Brushes.Gray });
                    break;
                case "code":
                    p.FontFamily = new FontFamily("Cascadia Mono, Consolas");
                    p.Background = new SolidColorBrush(Color.FromArgb(0x22, 0x80, 0x80, 0x80));
                    p.Padding = new Thickness(8);
                    break;
                case "quote":
                    p.FontStyle = FontStyles.Italic;
                    p.BorderBrush = Brushes.Gray;
                    p.BorderThickness = new Thickness(3, 0, 0, 0);
                    p.Padding = new Thickness(8, 0, 0, 0);
                    break;
            }
            foreach (var s in b.Spans) p.Inlines.Add(Inline(s));
            doc.Blocks.Add(p);
        }
        return doc;
    }

    private static Inline Inline(MdSpan s)
    {
        var run = new Run(s.Text);
        if (s.Bold == true) run.FontWeight = FontWeights.SemiBold;
        if (s.Italic == true) run.FontStyle = FontStyles.Italic;
        if (s.Code == true)
        {
            run.FontFamily = new FontFamily("Cascadia Mono, Consolas");
            run.Background = new SolidColorBrush(Color.FromArgb(0x22, 0x80, 0x80, 0x80));
        }
        if (s.Link is not { } link || !Uri.TryCreate(link, UriKind.Absolute, out var uri)) return run;
        var hyperlink = new Hyperlink(run) { NavigateUri = uri, ToolTip = link };
        hyperlink.Click += (_, _) => Process.Start(new ProcessStartInfo(uri.ToString()) { UseShellExecute = true });
        return hyperlink;
    }
}
