using System.IO;
using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;

namespace ODiagramador;

/// <summary>Brand marks for the MCP menu (Simple Icons, CC0 — see icons\README.md).</summary>
public static class BrandIcons
{
    private static readonly Dictionary<string, Geometry?> Cache = new();

    private static Geometry? Load(string name)
    {
        if (Cache.TryGetValue(name, out var g)) return g;
        Geometry? geometry = null;
        try
        {
            var svg = File.ReadAllText(System.IO.Path.Combine(AppContext.BaseDirectory, "icons", name + ".svg"));
            var d = Regex.Match(svg, "\\sd=\"([^\"]+)\"").Groups[1].Value;
            // SVG and WPF path mini-languages match; F1 = nonzero fill like SVG's default
            geometry = Geometry.Parse("F1 " + d);
            geometry.Freeze();
        }
        catch
        {
            // missing/odd file: caller falls back to a glyph
        }
        Cache[name] = geometry;
        return geometry;
    }

    /// <summary>24x24 brand path scaled to <paramref name="size"/>; null when the icon is missing.</summary>
    public static FrameworkElement? Create(string name, double size = 16, Brush? fill = null)
    {
        var geometry = Load(name);
        if (geometry is null) return null;
        var path = new System.Windows.Shapes.Path { Data = geometry, Stretch = Stretch.Uniform };
        if (fill is not null) path.Fill = fill;
        // monochrome marks take the text colour of the button / menu item they sit in (light and dark themes)
        else path.SetBinding(Shape.FillProperty, new System.Windows.Data.Binding("Foreground")
        {
            RelativeSource = new System.Windows.Data.RelativeSource(System.Windows.Data.RelativeSourceMode.FindAncestor, typeof(Control), 1),
        });
        return new Viewbox { Width = size, Height = size, Child = new Canvas { Width = 24, Height = 24, Children = { new Viewbox { Width = 24, Height = 24, Child = path } } } };
    }

    public static readonly Brush ClaudeOrange = Freeze(new SolidColorBrush(Color.FromRgb(0xD9, 0x77, 0x57)));

    private static Brush Freeze(Brush b)
    {
        b.Freeze();
        return b;
    }

    /// <summary>Segoe Fluent / MDL2 glyph as an icon.</summary>
    public static TextBlock Glyph(string glyph, double size = 16) => new()
    {
        Text = glyph,
        FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"),
        FontSize = size,
        VerticalAlignment = VerticalAlignment.Center,
    };

    /// <summary>Icon for an MCP client id (claude-code, codex, cursor, claude-desktop, kiro, antigravity, json).</summary>
    public static FrameworkElement ForClient(string id) => id switch
    {
        "claude-code" => Create("claudecode", 16, ClaudeOrange) ?? Glyph("\uE756"),
        "claude-desktop" => Create("claude", 16, ClaudeOrange) ?? Glyph("\uE8BD"),
        "cursor" => Create("cursor") ?? Glyph("\uE8B0"),
        "kiro" => Create("kiro") ?? Glyph("\uE99A"),
        "antigravity" => Glyph("\uE721"),
        "codex" => Glyph("\uE756"),
        _ => Glyph("\uE943"),
    };
}
