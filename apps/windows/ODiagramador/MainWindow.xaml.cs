using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using ICSharpCode.AvalonEdit.Document;
using ICSharpCode.AvalonEdit.Rendering;
using Microsoft.Web.WebView2.Core;
using Microsoft.Win32;

namespace ODiagramador;

public partial class MainWindow : Window
{
    private string? _workspace;
    private string _lang = System.Globalization.CultureInfo.CurrentUICulture.Name.StartsWith("pt") ? "pt-BR" : "en";
    private Dictionary<string, string> _strings = new();
    private string? _selected;
    private string _savedSource = "";
    private Report? _report;
    private string? _selectedElement;
    private bool _loading;
    private bool _viewerReady;
    private readonly List<string> _pending = new();
    private int _renderSeq;
    private string _renderSource = "";
    private bool _fitNext = true;
    private string? _lastSvg;
    private string? _lastPng;
    private readonly Dictionary<string, string> _modified = new();
    private readonly DispatcherTimer _poll = new() { Interval = TimeSpan.FromSeconds(1) };
    private readonly DispatcherTimer _renderDebounce = new() { Interval = TimeSpan.FromMilliseconds(300) };
    private readonly DispatcherTimer _saveDebounce = new() { Interval = TimeSpan.FromMilliseconds(900) };
    private readonly LineTint _tint = new();

    private static readonly string Settings = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "ODiagramador", "last-workspace");

    public MainWindow()
    {
        InitializeComponent();
        Editor.TextArea.TextView.BackgroundRenderers.Add(_tint);
        _poll.Tick += (_, _) => Poll();
        _renderDebounce.Tick += (_, _) => { _renderDebounce.Stop(); Render(); };
        _saveDebounce.Tick += (_, _) => { _saveDebounce.Stop(); FlushSave(); };
        LanguageBox.SelectedIndex = _lang == "pt-BR" ? 1 : 0;
        LoadStrings();
        Loaded += async (_, _) => await InitAsync();
        Closing += (_, _) => FlushSave();
    }

    private string T(string key) => _strings.TryGetValue(key, out var v) ? v : key;

    private void LoadStrings()
    {
        _strings = Core.CallOrDefault<Dictionary<string, string>>("strings", new { lang = _lang }) ?? new();
        OpenButton.Content = T("ui-open-folder");
        NewButton.Content = T("ui-new-diagram");
        ExportPngButton.Content = T("ui-export-png");
        ExportSvgButton.Content = T("ui-export-svg");
        McpMenu.Header = T("ui-copy-mcp-config");
        DiagramsHeader.Text = T("ui-diagrams").ToUpperInvariant();
        ProblemsTab.Header = $"{T("ui-problems")} ({_report?.Problems.Count ?? 0})";
        FeedbackTab.Header = T("ui-feedback");
        ActivityTab.Header = T("ui-activity");
        AddFeedbackButton.Content = T("ui-add-feedback");
        AgentBadge.Text = "✦ " + T("ui-changed-by-agent");
        EmptyText.Text = _workspace is null ? T("ui-no-workspace") : DiagramList.Items.Count == 0 ? T("ui-no-diagrams") : "";
        Send(new JsonObject { ["op"] = "strings", ["strings"] = new JsonObject { ["fit"] = T("ui-fit"), ["empty"] = T("ui-empty") } });
    }

    // ------------------------------------------------------------------ viewer

    private async Task InitAsync()
    {
        var kit = Path.Combine(AppContext.BaseDirectory, "render-kit");
        var env = await CoreWebView2Environment.CreateAsync(null,
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ODiagramador", "WebView2"));
        await Viewer.EnsureCoreWebView2Async(env);
        Viewer.CoreWebView2.SetVirtualHostNameToFolderMapping("odg.localhost", kit, CoreWebView2HostResourceAccessKind.Allow);
        Viewer.CoreWebView2.Settings.AreDefaultContextMenusEnabled = false;
        Viewer.CoreWebView2.WebMessageReceived += (_, e) => OnViewerMessage(e.TryGetWebMessageAsString());
        Viewer.CoreWebView2.Navigate("https://odg.localhost/index.html?mode=viewer");

        string? ws = null, open = null;
        for (var i = 0; i < App.Args.Length; i++)
        {
            if (App.Args[i] == "--workspace" && i + 1 < App.Args.Length) ws = App.Args[++i];
            else if (App.Args[i].StartsWith("odiagramador://")) (ws, open) = ParseOpenUrl(App.Args[i], ws);
        }
        ws ??= File.Exists(Settings) ? File.ReadAllText(Settings).Trim() : null;
        if (ws is not null && Directory.Exists(ws)) OpenWorkspace(ws);
        if (open is not null) Select(open);
        _poll.Start();
    }

    private static (string?, string?) ParseOpenUrl(string url, string? ws)
    {
        string? path = null;
        var q = url.Contains('?') ? url[(url.IndexOf('?') + 1)..] : "";
        foreach (var pair in q.Split('&'))
        {
            var kv = pair.Split('=', 2);
            var v = kv.Length > 1 ? Uri.UnescapeDataString(kv[1]) : "";
            if (kv[0] == "workspace") ws = v;
            if (kv[0] == "path") path = v;
        }
        return (ws, path);
    }

    private void Send(JsonObject msg)
    {
        var json = msg.ToJsonString();
        if (!_viewerReady || Viewer.CoreWebView2 is null)
        {
            if (msg["op"]?.GetValue<string>() == "process") _pending.RemoveAll(p => p.Contains("\"op\":\"process\""));
            _pending.Add(json);
            return;
        }
        _ = Viewer.CoreWebView2.ExecuteScriptAsync($"window.odg.handle({json});");
    }

    private void OnViewerMessage(string text)
    {
        var msg = JsonNode.Parse(text)?.AsObject();
        if (msg is null) return;
        switch (msg["op"]?.GetValue<string>())
        {
            case "ready":
                _viewerReady = true;
                foreach (var p in _pending.Where(p => !p.Contains("\"op\":\"process\"")).ToList())
                    _ = Viewer.CoreWebView2.ExecuteScriptAsync($"window.odg.handle({p});");
                _pending.Clear();
                LoadStrings();
                Render();
                break;
            case "result":
                RenderFinished(msg["id"]?.GetValue<int>() ?? 0, msg["result"]?.AsObject());
                break;
            case "select":
                _selectedElement = msg["element_id"]?.GetValue<string>();
                ElementTag.Text = _selectedElement ?? "";
                if (_selectedElement is not null &&
                    Core.CallOrDefault<int?>("find_line", new { source = Editor.Text, id = _selectedElement }) is int line)
                    ScrollToLine(line);
                break;
            case "select-problem":
                SelectProblem(msg["index"]?.GetValue<int>() ?? 0);
                break;
        }
    }

    private void Render()
    {
        if (_selected is null)
        {
            Send(new JsonObject { ["op"] = "process", ["id"] = 0, ["source"] = "" });
            return;
        }
        _renderSeq++;
        _renderSource = Editor.Text;
        Send(new JsonObject
        {
            ["op"] = "process", ["id"] = _renderSeq, ["source"] = _renderSource,
            ["theme"] = "default", ["png"] = true, ["scale"] = 2,
        });
    }

    private void RenderFinished(int id, JsonObject? result)
    {
        if (id != _renderSeq || result is null || _selected is null || _workspace is null) return;
        _lastSvg = result["svg"]?.GetValue<string>();
        _lastPng = result["png"]?.GetValue<string>();
        try
        {
            _report = Core.CallNode<Report>("analyze", new JsonObject
            {
                ["workspace"] = _workspace, ["path"] = _selected, ["source"] = _renderSource,
                ["raw"] = JsonNode.Parse(result.ToJsonString()), ["lang"] = _lang, ["save"] = _renderSource == _savedSource,
            });
        }
        catch (Exception ex)
        {
            StatusText.Text = ex.Message;
            return;
        }
        var problems = new JsonArray();
        foreach (var p in _report.Problems)
        {
            var o = new JsonObject { ["index"] = p.Index, ["severity"] = p.Severity, ["code"] = p.Code, ["message"] = p.Message };
            if (p.Bbox is not null)
                o["bbox"] = new JsonObject { ["x"] = p.Bbox.X, ["y"] = p.Bbox.Y, ["w"] = p.Bbox.W, ["h"] = p.Bbox.H };
            problems.Add(o);
        }
        Send(new JsonObject { ["op"] = "show", ["problems"] = problems });
        if (_fitNext)
        {
            Send(new JsonObject { ["op"] = "fit" });
            _fitNext = false;
        }
        ProblemList.ItemsSource = _report.Problems;
        ProblemsTab.Header = $"{T("ui-problems")} ({_report.Problems.Count})";
        StatusText.Text = _report.Summary;
        _tint.Errors = _report.Problems.Where(p => p.Severity == "error" && p.Line is not null).Select(p => p.Line!.Value).ToHashSet();
        _tint.Warnings = _report.Problems.Where(p => p.Severity == "warning" && p.Line is not null).Select(p => p.Line!.Value).ToHashSet();
        Editor.TextArea.TextView.InvalidateLayer(KnownLayer.Background);
    }

    // ------------------------------------------------------------------ workspace

    private void OpenWorkspace(string dir)
    {
        _workspace = Path.GetFullPath(dir);
        Directory.CreateDirectory(Path.GetDirectoryName(Settings)!);
        File.WriteAllText(Settings, _workspace);
        if (Core.CallOrDefault<WorkspaceConfig>("config", new { workspace = _workspace })?.Lang is string l && l != _lang)
        {
            _lang = l;
            LanguageBox.SelectedIndex = l == "pt-BR" ? 1 : 0;
        }
        Title = $"ODiagramador — {_workspace}";
        _modified.Clear();
        _selected = null;
        RefreshList();
        LoadStrings();
        if (DiagramList.Items.Count > 0) DiagramList.SelectedIndex = 0;
    }

    private void RefreshList()
    {
        if (_workspace is null) return;
        var list = Core.CallOrDefault<List<DiagramInfo>>("list", new { workspace = _workspace }) ?? new();
        var keep = _selected;
        DiagramList.ItemsSource = list;
        if (keep is not null) DiagramList.SelectedItem = list.FirstOrDefault(d => d.Path == keep);
        foreach (var d in list) _modified.TryAdd(d.Path, d.Modified);
        RefreshSide();
    }

    private void RefreshSide()
    {
        if (_workspace is null) return;
        FeedbackList.ItemsSource = Core.CallOrDefault<List<Feedback>>("feedback",
            new { workspace = _workspace, path = _selected, include_resolved = true });
        ActivityList.ItemsSource = Core.CallOrDefault<List<ActivityEvent>>("events", new { workspace = _workspace, limit = 40 });
    }

    private void Poll()
    {
        if (_workspace is null) return;
        var list = Core.CallOrDefault<List<DiagramInfo>>("list", new { workspace = _workspace });
        if (list is null) return;
        var changed = list.Where(d => !_modified.TryGetValue(d.Path, out var m) || m != d.Modified).Select(d => d.Path).ToList();
        var current = (DiagramList.ItemsSource as List<DiagramInfo>) ?? new();
        var structure = !list.Select(d => (d.Path, d.OpenFeedback)).SequenceEqual(current.Select(d => (d.Path, d.OpenFeedback)));
        if (changed.Count == 0 && !structure) return;
        foreach (var d in list) _modified[d.Path] = d.Modified;
        _loading = true;
        RefreshList();
        _loading = false;
        if (_selected is not null && changed.Contains(_selected))
        {
            var disk = Core.CallOrDefault<string>("read", new { workspace = _workspace, path = _selected }) ?? "";
            if (disk != _savedSource)
            {
                var last = Core.CallOrDefault<List<ActivityEvent>>("events", new { workspace = _workspace, limit = 1, path = _selected });
                AgentBadge.Visibility = last?.FirstOrDefault()?.Actor == "agent" ? Visibility.Visible : Visibility.Collapsed;
                _savedSource = disk;
                SetText(disk);
                Render();
            }
        }
    }

    private void SetText(string text)
    {
        _loading = true;
        Editor.Document.Text = text;
        _loading = false;
    }

    private void Select(string path)
    {
        FlushSave();
        if (_workspace is null) return;
        _selected = path;
        _selectedElement = null;
        ElementTag.Text = "";
        _fitNext = true;
        AgentBadge.Visibility = Visibility.Collapsed;
        _savedSource = Core.CallOrDefault<string>("read", new { workspace = _workspace, path }) ?? "";
        SetText(_savedSource);
        RefreshSide();
        Render();
    }

    private void FlushSave()
    {
        _saveDebounce.Stop();
        if (_workspace is null || _selected is null || Editor.Text == _savedSource) return;
        try
        {
            Core.Call<string>("write", new { workspace = _workspace, path = _selected, content = Editor.Text, actor = "human" });
            _savedSource = Editor.Text;
            Render();
        }
        catch (Exception ex)
        {
            StatusText.Text = ex.Message;
        }
    }

    private void ScrollToLine(int line)
    {
        if (line < 1 || line > Editor.Document.LineCount) return;
        var l = Editor.Document.GetLineByNumber(line);
        Editor.ScrollToLine(line);
        Editor.TextArea.Caret.Offset = l.Offset;
        Editor.Select(l.Offset, l.Length);
    }

    private void SelectProblem(int index)
    {
        var p = _report?.Problems.FirstOrDefault(x => x.Index == index);
        if (p is null) return;
        if (p.Line is int line) ScrollToLine(line);
        if (p.ElementId is not null)
        {
            _selectedElement = p.ElementId;
            ElementTag.Text = p.ElementId;
            Send(new JsonObject { ["op"] = "highlight", ["element_id"] = p.ElementId });
        }
    }

    // ------------------------------------------------------------------ UI events

    private void OnOpenFolder(object sender, RoutedEventArgs e)
    {
#if NET8_0_OR_GREATER
        var dialog = new OpenFolderDialog { Title = T("ui-open-folder") };
        if (dialog.ShowDialog(this) == true) OpenWorkspace(dialog.FolderName);
#else
        var path = Prompt.Ask(this, T("ui-open-folder"), T("ui-open-folder"), "OK", T("ui-cancel"));
        if (!string.IsNullOrWhiteSpace(path) && Directory.Exists(path)) OpenWorkspace(path);
#endif
    }

    private void OnNewDiagram(object sender, RoutedEventArgs e)
    {
        if (_workspace is null) return;
        var name = Prompt.Ask(this, T("ui-new-diagram"), T("ui-new-diagram-name"), T("ui-create"), T("ui-cancel"));
        if (string.IsNullOrWhiteSpace(name)) return;
        var slug = string.Join("-", System.Text.RegularExpressions.Regex.Split(name.ToLowerInvariant(), @"[^\p{L}\p{N}]+").Where(p => p.Length > 0));
        var path = $"diagrams/{(slug.Length == 0 ? "diagram" : slug)}.mmd";
        var rel = Core.Call<string>("write", new
        {
            workspace = _workspace, path, actor = "human",
            content = "flowchart LR\n    start([Start]) --> step[Do something]\n    step --> done([Done])\n",
        });
        RefreshList();
        DiagramList.SelectedItem = (DiagramList.ItemsSource as List<DiagramInfo>)?.FirstOrDefault(d => d.Path == rel);
    }

    private void OnDiagramSelected(object sender, SelectionChangedEventArgs e)
    {
        if (_loading) return;
        if (DiagramList.SelectedItem is DiagramInfo d && d.Path != _selected) Select(d.Path);
    }

    private void OnSourceChanged(object? sender, EventArgs e)
    {
        if (_loading) return;
        _renderDebounce.Stop();
        _renderDebounce.Start();
        _saveDebounce.Stop();
        _saveDebounce.Start();
    }

    private void OnProblemSelected(object sender, SelectionChangedEventArgs e)
    {
        if (ProblemList.SelectedItem is Problem p) SelectProblem(p.Index);
    }

    private void OnAddFeedback(object sender, RoutedEventArgs e)
    {
        if (_workspace is null || _selected is null || string.IsNullOrWhiteSpace(FeedbackBox.Text)) return;
        Core.Call<Feedback>("add_feedback", new
        {
            workspace = _workspace, path = _selected, text = FeedbackBox.Text,
            element_id = _selectedElement, author = Environment.UserName,
        });
        FeedbackBox.Text = "";
        RefreshList();
    }

    private void OnFeedbackKey(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter) OnAddFeedback(sender, e);
    }

    private void OnResolveFeedback(object sender, RoutedEventArgs e)
    {
        if (_workspace is null || (sender as Button)?.Tag is not string id) return;
        Core.Run("resolve_feedback", new { workspace = _workspace, id, actor = "human" });
        RefreshList();
    }

    private void OnLanguageChanged(object sender, SelectionChangedEventArgs e)
    {
        if ((LanguageBox.SelectedItem as ComboBoxItem)?.Tag is not string lang || lang == _lang) return;
        _lang = lang;
        if (_workspace is not null) Core.Run("set_config", new { workspace = _workspace, lang });
        LoadStrings();
        Render();
    }

    private void OnExportPng(object sender, RoutedEventArgs e) => Export(png: true);

    private void OnExportSvg(object sender, RoutedEventArgs e) => Export(png: false);

    private void Export(bool png)
    {
        if (_selected is null) return;
        byte[]? data = png
            ? (_lastPng is null ? null : Convert.FromBase64String(_lastPng))
            : (_lastSvg is null ? null : System.Text.Encoding.UTF8.GetBytes(_lastSvg));
        if (data is null) return;
        var dialog = new SaveFileDialog
        {
            FileName = Path.GetFileNameWithoutExtension(_selected) + (png ? ".png" : ".svg"),
            Filter = png ? "PNG|*.png" : "SVG|*.svg",
        };
        if (dialog.ShowDialog(this) == true) File.WriteAllBytes(dialog.FileName, data);
    }

    private void OnCopyMcp(object sender, RoutedEventArgs e)
    {
        if (_workspace is null || (sender as MenuItem)?.Tag is not string kind) return;
        var odg = Path.Combine(AppContext.BaseDirectory, "odg.exe");
        var cfg = Core.Call<McpConfig>("mcp_config", new { odg_path = odg, workspace = _workspace });
        Clipboard.SetText(kind switch { "claude" => cfg.ClaudeCode, "codex" => cfg.Codex, _ => cfg.Json });
        StatusText.Text = T("ui-copied");
    }

    /// <summary>Tints lines that have errors / warnings in the editor.</summary>
    private sealed class LineTint : IBackgroundRenderer
    {
        public HashSet<int> Errors { get; set; } = new();
        public HashSet<int> Warnings { get; set; } = new();
        public KnownLayer Layer => KnownLayer.Background;

        private static readonly Brush ErrorBrush = Freeze(new SolidColorBrush(Color.FromArgb(56, 209, 36, 47)));
        private static readonly Brush WarningBrush = Freeze(new SolidColorBrush(Color.FromArgb(40, 217, 119, 6)));

        private static Brush Freeze(Brush b)
        {
            b.Freeze();
            return b;
        }

        public void Draw(TextView textView, DrawingContext dc)
        {
            foreach (var vl in textView.VisualLines)
            {
                var n = vl.FirstDocumentLine.LineNumber;
                var brush = Errors.Contains(n) ? ErrorBrush : Warnings.Contains(n) ? WarningBrush : null;
                if (brush is null) continue;
                var y = vl.VisualTop - textView.ScrollOffset.Y;
                dc.DrawRectangle(brush, null, new Rect(0, y, textView.ActualWidth, vl.Height));
            }
        }
    }
}

/// <summary>Minimal input box (WPF has none built in).</summary>
internal static class Prompt
{
    public static string? Ask(Window owner, string title, string label, string ok, string cancel)
    {
        var box = new TextBox { Margin = new Thickness(0, 6, 0, 10), MinWidth = 280 };
        var okButton = new Button { Content = ok, IsDefault = true, Padding = new Thickness(12, 2, 12, 2), Margin = new Thickness(0, 0, 6, 0) };
        var cancelButton = new Button { Content = cancel, IsCancel = true, Padding = new Thickness(12, 2, 12, 2) };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
        buttons.Children.Add(okButton);
        buttons.Children.Add(cancelButton);
        var panel = new StackPanel { Margin = new Thickness(14) };
        panel.Children.Add(new TextBlock { Text = label });
        panel.Children.Add(box);
        panel.Children.Add(buttons);
        var win = new Window
        {
            Title = title, Owner = owner, Content = panel, SizeToContent = SizeToContent.WidthAndHeight,
            ResizeMode = ResizeMode.NoResize, WindowStartupLocation = WindowStartupLocation.CenterOwner,
        };
        okButton.Click += (_, _) => win.DialogResult = true;
        box.Loaded += (_, _) => box.Focus();
        return win.ShowDialog() == true ? box.Text : null;
    }
}
