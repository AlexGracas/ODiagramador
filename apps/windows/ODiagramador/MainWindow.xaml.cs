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
    /// <summary>Read position in the agents' message log (null = viewer just opened on this folder).</summary>
    private ulong? _inboxCursor;
    private DateTime _presenceAt = DateTime.MinValue;
    private readonly DispatcherTimer _renderDebounce = new() { Interval = TimeSpan.FromMilliseconds(300) };
    private readonly DispatcherTimer _saveDebounce = new() { Interval = TimeSpan.FromMilliseconds(900) };
    private readonly LineTint _tint = new();
    internal readonly Preferences Prefs = App.Prefs;
    internal readonly RemoteMcp Remote;
    internal readonly Updater Updates = new();
    private string _diagramFont = "";
    private readonly DispatcherTimer _toastTimer = new() { Interval = TimeSpan.FromSeconds(2.5) };

    internal static readonly (string Id, string Name)[] McpClients =
    {
        ("claude-code", "Claude Code"), ("codex", "Codex"), ("cursor", "Cursor"), ("claude-desktop", "Claude Desktop"),
    };

    public MainWindow()
    {
        InitializeComponent();
        Remote = new RemoteMcp(Prefs);
        _lang = Prefs.Language ?? _lang;
        Editor.TextArea.TextView.BackgroundRenderers.Add(_tint);
        McpIcon.Content = BrandIcons.Create("modelcontextprotocol") ?? BrandIcons.Glyph("\uE8F1");
        App.Log($"main window: icons {(McpIcon.Content is TextBlock ? "missing (glyph fallback)" : "ok")}, base {AppContext.BaseDirectory}");
        // trace clicks and menus: "the button does nothing" reports need to know how far a click got
        foreach (var b in new[] { OpenButton, NewButton, ExportButton, McpButton, UpdateButton, SettingsButton })
            b.PreviewMouseLeftButtonDown += (_, _) => App.Log($"mouse down on {b.Name}");
        McpMenu.Opened += (_, _) => App.Log($"MCP menu opened ({McpMenu.Items.Count} items)");
        McpMenu.Closed += (_, _) => App.Log("MCP menu closed");
        if (ExportButton.ContextMenu is { } exportMenu) exportMenu.Opened += (_, _) => App.Log("export menu opened");
        _toastTimer.Tick += (_, _) => { _toastTimer.Stop(); Toast.Visibility = Visibility.Collapsed; };
        SystemEvents.UserPreferenceChanged += OnSystemPreferenceChanged;
        Closed += (_, _) => SystemEvents.UserPreferenceChanged -= OnSystemPreferenceChanged;
        ApplyEditorAppearance();
        _poll.Tick += (_, _) => Poll();
        _renderDebounce.Tick += (_, _) => { _renderDebounce.Stop(); Render(); };
        _saveDebounce.Tick += (_, _) => { _saveDebounce.Stop(); FlushSave(); };
        LoadStrings();
        Loaded += async (_, _) => await InitAsync();
        Closing += (_, _) =>
        {
            FlushSave();
            // agents should no longer count on the human seeing their messages
            if (_workspace is not null) Core.CallOrDefault<JsonNode>("clear_presence", new { workspace = _workspace });
            Remote.Stop();
            Prefs.Save();
        };
    }

    internal string T(string key) => _strings.TryGetValue(key, out var v) ? v : key;

    /// <summary>Translation with Fluent arguments ({ $name }).</summary>
    internal string T(string key, object args) =>
        Core.CallOrDefault<string>("t", new { lang = _lang, key, args }) ?? key;

    internal string Lang => _lang;

    private void LoadStrings()
    {
        _strings = Core.CallOrDefault<Dictionary<string, string>>("strings", new { lang = _lang }) ?? new();
        OpenButton.ToolTip = T("ui-open-folder");
        NewButton.ToolTip = T("ui-new-diagram");
        ExportButton.ToolTip = T("ui-export");
        McpButton.ToolTip = T("ui-mcp-configure");
        SettingsButton.ToolTip = T("ui-settings");
        BuildMcpMenu();
        DiagramsHeader.Text = T("ui-diagrams").ToUpperInvariant();
        ProblemsTab.Header = $"{T("ui-problems")} ({_report?.Problems.Count ?? 0})";
        FeedbackTab.Header = T("ui-feedback");
        ActivityTab.Header = T("ui-activity");
        AddFeedbackButton.Content = T("ui-add-feedback");
        AgentBadge.Text = "✦ " + T("ui-changed-by-agent");
        EmptyText.Text = _workspace is null ? T("ui-no-workspace") : DiagramList.Items.Count == 0 ? T("ui-no-diagrams") : "";
        var all = new JsonObject();
        foreach (var (k, v) in _strings) all[k] = v;
        Send(new JsonObject { ["op"] = "strings", ["strings"] = all });
    }

    // ------------------------------------------------------------------ viewer

    private async Task InitAsync()
    {
        var kit = Path.Combine(AppContext.BaseDirectory, "render-kit");
        App.Log($"init: render-kit {(Directory.Exists(kit) ? "found" : "MISSING")}, odg.exe {(File.Exists(Path.Combine(AppContext.BaseDirectory, "odg.exe")) ? "found" : "MISSING")}, prefs lang={Prefs.Language} appearance={Prefs.Appearance} dark={Prefs.IsDark}");
        var env = await CoreWebView2Environment.CreateAsync(null,
            Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ODiagramador", "WebView2"));
        await Viewer.EnsureCoreWebView2Async(env);
        App.Log($"init: WebView2 {env.BrowserVersionString}");
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
        ws ??= Prefs.LastWorkspace;
        if (ws is not null && Directory.Exists(ws)) OpenWorkspace(ws);
        if (open is not null) Select(open);
        _poll.Start();
        if (Prefs.AutoUpdate && DateTime.UtcNow - Prefs.LastUpdateCheck > TimeSpan.FromHours(24))
            await CheckForUpdatesAsync(userInitiated: false);
    }

    // ------------------------------------------------------------------ menus, MCP, updates

    private void OnOpenMenu(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { ContextMenu: { } menu } button) return;
        App.Log($"open menu {button.Name} ({menu.Items.Count} items)");
        menu.PlacementTarget = button;
        menu.Placement = System.Windows.Controls.Primitives.PlacementMode.Bottom;
        menu.IsOpen = true;
    }

    private void BuildMcpMenu()
    {
        McpMenu.Items.Clear();
        McpMenu.Items.Add(new MenuItem { Header = T("ui-mcp-copy-for"), IsEnabled = false });
        foreach (var (id, name) in McpClients)
        {
            var item = new MenuItem { Header = name, Icon = BrandIcons.ForClient(id), Tag = id };
            item.Click += (_, _) =>
            {
                App.Log($"MCP menu item clicked: {id}");
                CopyMcp(id, remote: false);
            };
            McpMenu.Items.Add(item);
        }
        var other = new MenuItem { Header = T("ui-mcp-other"), Icon = BrandIcons.ForClient("json") };
        other.Click += (_, _) =>
        {
            App.Log("MCP menu item clicked: json");
            CopyMcp("json", remote: false);
        };
        McpMenu.Items.Add(other);
        McpMenu.Items.Add(new Separator());
        var settings = new MenuItem { Header = T("ui-mcp-settings"), Icon = BrandIcons.Glyph("\uE713") };
        settings.Click += (_, _) =>
        {
            App.Log("MCP menu item clicked: settings");
            OpenSettings("mcp");
        };
        McpMenu.Items.Add(settings);
    }

    /// <summary>
    /// Copies the MCP configuration for a client and always shows it in a dialog (over the window that was clicked)
    /// with the next step for that client, so the result is visible even when the clipboard fails.
    /// </summary>
    internal void CopyMcp(string client, bool remote, Window? owner = null)
    {
        owner ??= this;
        App.Log($"copy MCP config: client={client} remote={remote} workspace={_workspace ?? "(none)"}");
        if (_workspace is null)
        {
            if (MessageBox.Show(owner, T("ui-mcp-need-folder"), "MCP", MessageBoxButton.OKCancel, MessageBoxImage.Information) == MessageBoxResult.OK)
            {
                OnOpenFolder(this, new RoutedEventArgs());
                if (_workspace is not null) CopyMcp(client, remote, owner);
            }
            return;
        }
        string text;
        try
        {
            var args = new JsonObject { ["odg_path"] = Path.Combine(AppContext.BaseDirectory, "odg.exe"), ["workspace"] = _workspace };
            if (remote)
            {
                args["url"] = Remote.Url;
                args["token"] = Prefs.McpToken;
            }
            var cfg = Core.CallNode<McpConfig>("mcp_config", args);
            text = (client, remote) switch
            {
                ("claude-code", false) => cfg.ClaudeCode,
                ("codex", false) => cfg.Codex,
                ("claude-code", true) => cfg.RemoteClaudeCode ?? cfg.ClaudeCode,
                ("codex", true) => cfg.RemoteCodex ?? cfg.Codex,
                (_, true) => cfg.RemoteJson ?? cfg.Json,
                _ => cfg.Json,
            };
        }
        catch (Exception ex)
        {
            App.Log("mcp_config failed: " + ex);
            MessageBox.Show(owner, ex.Message, "ODiagramador", MessageBoxButton.OK, MessageBoxImage.Error);
            return;
        }
        var name = McpClients.FirstOrDefault(c => c.Id == client).Name ?? T("ui-mcp-other");
        var copied = TrySetClipboard(text);
        var howto = T(client switch
        {
            "claude-code" or "codex" or "cursor" or "claude-desktop" => "ui-mcp-howto-" + client,
            _ => "ui-mcp-howto-json",
        });
        Prompt.McpConfig(owner, name, copied ? "✓  " + T("ui-mcp-copied", new { app = name }) : T("ui-mcp-copy-failed"),
            T("ui-mcp-next-step") + " " + howto, text, T("ui-mcp-copy-again"), () => TrySetClipboard(text));
    }

    /// <summary>The Windows clipboard is often locked briefly by other apps: retry instead of failing.</summary>
    private static bool TrySetClipboard(string text)
    {
        Exception? last = null;
        for (var i = 0; i < 10; i++)
        {
            try
            {
                Clipboard.SetDataObject(text, true);
                var back = Clipboard.ContainsText() ? Clipboard.GetText() : null;
                App.Log($"clipboard set on attempt {i + 1}: {text.Length} chars, read back {(back == text ? "matches" : back is null ? "NO TEXT" : $"DIFFERENT ({back.Length} chars)")}");
                return true;
            }
            catch (Exception ex) when (ex is System.Runtime.InteropServices.ExternalException or InvalidOperationException)
            {
                last = ex;
                Thread.Sleep(60);
            }
        }
        App.Log("clipboard failed: " + last);
        return false;
    }

    internal void ShowToast(string text)
    {
        ToastText.Text = "✓  " + text;
        Toast.Visibility = Visibility.Visible;
        _toastTimer.Stop();
        _toastTimer.Start();
    }

    private void OnSettings(object sender, RoutedEventArgs e) => OpenSettings("general");

    private SettingsWindow? _settings;

    internal void OpenSettings(string tab)
    {
        App.Log($"open settings: {tab}");
        if (_settings is { IsLoaded: true })
        {
            _settings.SelectTab(tab);
            _settings.Activate();
            return;
        }
        _settings = new SettingsWindow(this, tab) { Owner = this };
        _settings.Show();
    }

    internal async Task CheckForUpdatesAsync(bool userInitiated)
    {
        try
        {
            var info = await Updates.CheckAsync();
            App.Log($"update check: current {info?.Current} latest {info?.Latest} available={info?.Available} asset={info?.AssetName}");
            Prefs.LastUpdateCheck = DateTime.UtcNow;
            Prefs.Save();
            UpdateButton.Visibility = Updates.Available is null ? Visibility.Collapsed : Visibility.Visible;
            if (Updates.Available is not null)
            {
                UpdateButton.ToolTip = T("ui-update-available", new { version = info!.Latest, current = info.Current });
                if (!userInitiated && Prefs.AutoInstallUpdates) await InstallUpdateAsync();
                else ShowUpdateDialog();
            }
            else if (userInitiated)
            {
                MessageBox.Show(this, T("ui-up-to-date", new { version = Updater.CurrentVersion }), "ODiagramador");
            }
        }
        catch (Exception ex)
        {
            App.Log("update check failed: " + ex);
            if (userInitiated) MessageBox.Show(this, T("ui-update-failed", new { detail = ex.Message }), "ODiagramador");
        }
    }

    private void OnShowUpdate(object sender, RoutedEventArgs e) => ShowUpdateDialog();

    private void ShowUpdateDialog()
    {
        var info = Updates.Available;
        if (info is null) return;
        var choice = Prompt.Update(this, T("ui-update-available", new { version = info.Latest, current = info.Current }), info.Notes,
            T("ui-update-now"), T("ui-update-later"), T("ui-update-release-page"));
        if (choice == "install") _ = InstallUpdateAsync();
        else if (choice == "page") System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(info.PageUrl) { UseShellExecute = true });
    }

    private async Task InstallUpdateAsync()
    {
        try
        {
            ShowToast(T("ui-updating"));
            FlushSave();
            Remote.Stop();
            await Updates.InstallAsync(_ => ShowToast(T("ui-update-restart")));
        }
        catch (UnauthorizedAccessException)
        {
            MessageBox.Show(this, T("ui-update-manual"), "ODiagramador");
            if (Updates.Available is { } info)
                System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(info.PageUrl) { UseShellExecute = true });
        }
        catch (Exception ex)
        {
            App.Log("update install failed: " + ex);
            MessageBox.Show(this, T("ui-update-failed", new { detail = ex.Message }), "ODiagramador");
        }
    }

    // ------------------------------------------------------------------ appearance & fonts

    internal void ApplyEditorAppearance()
    {
        var dark = Prefs.IsDark;
        App.ApplyTheme(Prefs);
        Editor.FontFamily = new FontFamily(Prefs.EditorFont + ", Cascadia Mono, Consolas");
        Editor.FontSize = Prefs.EditorFontSize;
        Editor.Background = dark ? new SolidColorBrush(Color.FromRgb(0x1E, 0x1E, 0x1E)) : Brushes.White;
        Editor.Foreground = dark ? new SolidColorBrush(Color.FromRgb(0xD4, 0xD4, 0xD4)) : Brushes.Black;
        Editor.LineNumbersForeground = dark ? Brushes.Gray : Brushes.DimGray;
        Send(new JsonObject { ["op"] = "theme", ["theme"] = dark ? "dark" : "light" });
        Render();
    }

    private void OnSystemPreferenceChanged(object sender, UserPreferenceChangedEventArgs e)
    {
        if (e.Category == UserPreferenceCategory.General && Prefs.Appearance == "system")
            Dispatcher.BeginInvoke(ApplyEditorAppearance);
    }

    internal string DiagramFont
    {
        get => _diagramFont;
        set
        {
            _diagramFont = value;
            if (_workspace is not null) Core.Run("set_config", new { workspace = _workspace, font = value });
            Render();
        }
    }

    internal void SetLanguage(string lang)
    {
        if (lang == _lang) return;
        _lang = lang;
        Prefs.Language = lang;
        Prefs.Save();
        if (_workspace is not null) Core.Run("set_config", new { workspace = _workspace, lang });
        LoadStrings();
        Render();
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
            case "answer":
                AnswerAgent(msg["question_id"]?.GetValue<string>(), msg["text"]?.GetValue<string>());
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
            case "move":
                ApplyLayoutEdit(new JsonObject
                {
                    ["op"] = "move", ["id"] = msg["element_id"]?.GetValue<string>(),
                    ["x"] = msg["x"]?.GetValue<double>(), ["y"] = msg["y"]?.GetValue<double>(),
                });
                break;
            case "layout-mode":
                ApplyLayoutEdit(new JsonObject { ["op"] = "mode", ["mode"] = msg["mode"]?.GetValue<string>() });
                break;
            case "reset-positions":
                ApplyLayoutEdit(new JsonObject { ["op"] = "reset" });
                break;
            case "edit":
                ApplySourceEdit("edit", new JsonObject
                {
                    ["action"] = msg["action"]?.GetValue<string>(),
                    ["params"] = msg["params"] is JsonNode ep ? JsonNode.Parse(ep.ToJsonString()) : new JsonObject(),
                });
                break;
            case "construct":
                ApplySourceEdit("construct", new JsonObject
                {
                    ["construct"] = msg["construct"]?.GetValue<string>(),
                    ["params"] = msg["params"] is JsonNode prm ? JsonNode.Parse(prm.ToJsonString()) : new JsonObject(),
                });
                break;
        }
    }

    /// <summary>Drag &amp; drop / layout menu: rewrite the %% @odg comments via the core, save, re-render.</summary>
    private void ApplyLayoutEdit(JsonObject args) => ApplySourceEdit("layout_edit", args);

    /// <summary>Palette constructors and layout edits: the core returns the new source.</summary>
    private void ApplySourceEdit(string method, JsonObject args)
    {
        if (_selected is null) return;
        args["source"] = Editor.Text;
        try
        {
            var updated = Core.CallNode<string>(method, args);
            if (updated == Editor.Text) return;
            Editor.Document.Text = updated;
            FlushSave();
        }
        catch (Exception ex)
        {
            App.Log("error: " + ex);
            StatusText.Text = ex.Message;
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
            ["theme"] = Prefs.IsDark ? "dark" : "default", ["png"] = true, ["scale"] = 2, ["font"] = _diagramFont,
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
            App.Log("error: " + ex);
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
        if (_workspace is not null && _workspace != Path.GetFullPath(dir))
            Core.CallOrDefault<JsonNode>("clear_presence", new { workspace = _workspace });
        _workspace = Path.GetFullPath(dir);
        _inboxCursor = null;
        _presenceAt = DateTime.MinValue;
        App.Log($"open workspace: {_workspace}");
        Prefs.LastWorkspace = _workspace;
        Prefs.Save();
        var cfg = Core.CallOrDefault<WorkspaceConfig>("config", new { workspace = _workspace });
        if (cfg?.Lang is string l && l != _lang) _lang = l;
        _diagramFont = cfg?.Font ?? "";
        Remote.Configure(_workspace, Path.Combine(AppContext.BaseDirectory, "odg.exe"));
        Title = $"ODiagramador — {_workspace}";
        TitleText.Text = _workspace;
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
        SyncAgents();
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

    // ------------------------------------------------------------------ agent ⇄ human (MCP show_diagram, highlight_elements, notify_user, ask_user)

    /// <summary>Tells agents what the human is looking at and delivers their messages to the viewer.</summary>
    private void SyncAgents()
    {
        if (_workspace is null) return;
        try
        {
            if (DateTime.UtcNow - _presenceAt >= TimeSpan.FromSeconds(2))
            {
                _presenceAt = DateTime.UtcNow;
                Core.Run("set_presence", new
                {
                    workspace = _workspace, app = "ODiagramador", platform = "windows", version = Updater.CurrentVersion,
                    path = _selected, element_id = _selectedElement,
                });
            }
            var inbox = Core.Call<JsonNode>("viewer_inbox", new { workspace = _workspace, cursor = _inboxCursor });
            _inboxCursor = inbox["cursor"]?.GetValue<ulong>();
            var messages = inbox["messages"]?.AsArray() ?? new JsonArray();
            foreach (var m in messages)
                if (m is JsonObject o) DeliverAgentMessage(o);
            if (messages.Count > 0) RefreshSide();
        }
        catch (Exception ex)
        {
            App.Log("agent sync failed: " + ex.Message);
        }
    }

    private void DeliverAgentMessage(JsonObject m)
    {
        var kind = m["kind"]?.GetValue<string>() ?? "";
        var hasIds = m["element_ids"] is JsonArray { Count: > 0 };
        var path = m["path"]?.GetValue<string>();
        App.Log($"agent message: {kind} {path} {m["id"]}");
        var switched = false;
        if (path is not null && (kind is "show" or "highlight" || hasIds) && path != _selected)
        {
            if ((DiagramList.ItemsSource as List<DiagramInfo>)?.All(d => d.Path != path) ?? true) RefreshList();
            if ((DiagramList.ItemsSource as List<DiagramInfo>)?.FirstOrDefault(d => d.Path == path) is { } item)
            {
                Select(path);
                _loading = true;
                DiagramList.SelectedItem = item;
                _loading = false;
                switched = true;
            }
        }
        Send(new JsonObject { ["op"] = "agent", ["message"] = m.DeepClone(), ["switched"] = switched });
        if (kind == "notify") return;
        // show / highlight / ask: bring the window forward so the human sees it
        if (WindowState == WindowState.Minimized) WindowState = WindowState.Normal;
        Activate();
        Topmost = true;
        Topmost = false;
    }

    private void AnswerAgent(string? questionId, string? text)
    {
        if (_workspace is null || questionId is null || string.IsNullOrWhiteSpace(text)) return;
        try
        {
            Core.Run("answer", new { workspace = _workspace, question_id = questionId, text });
            App.Log($"answered {questionId}");
        }
        catch (Exception ex)
        {
            App.Log("answer failed: " + ex);
            StatusText.Text = ex.Message;
        }
        RefreshSide();
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
            App.Log("error: " + ex);
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
        var kinds = new[] { "flowchart", "sequence", "class", "state", "er" };
        var answer = Prompt.AskWithChoice(this, T("ui-new-diagram"), T("ui-new-diagram-type"),
            kinds.Select(k => (k, T("ui-type-" + k))).ToArray(), T("ui-new-diagram-name"), T("ui-create"), T("ui-cancel"));
        if (answer is null || string.IsNullOrWhiteSpace(answer.Value.Text)) return;
        var (kind, name) = answer.Value;
        var slug = string.Join("-", System.Text.RegularExpressions.Regex.Split(name.ToLowerInvariant(), @"[^\p{L}\p{N}]+").Where(p => p.Length > 0));
        var path = $"diagrams/{(slug.Length == 0 ? "diagram" : slug)}.mmd";
        var rel = Core.Call<string>("write", new
        {
            workspace = _workspace, path, actor = "human",
            content = Core.CallOrDefault<string>("template", new { kind, lang = _lang }) ?? "flowchart LR\n    start([Start]) --> done([Done])\n",
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

    private void OnExportPng(object sender, RoutedEventArgs e) => Export(png: true);

    private void OnExportSvg(object sender, RoutedEventArgs e) => Export(png: false);

    private void Export(bool png)
    {
        App.Log($"export {(png ? "png" : "svg")}: selected={_selected ?? "(none)"} png={_lastPng is not null} svg={_lastSvg is not null}");
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
        if (dialog.ShowDialog(this) != true) return;
        File.WriteAllBytes(dialog.FileName, data);
        App.Log($"exported {data.Length} bytes to {dialog.FileName}");
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
    public static (string Choice, string Text)? AskWithChoice(Window owner, string title, string choiceLabel,
        (string Value, string Label)[] choices, string label, string ok, string cancel)
    {
        var combo = new ComboBox { Margin = new Thickness(0, 6, 0, 10), MinWidth = 280 };
        foreach (var c in choices) combo.Items.Add(new ComboBoxItem { Content = c.Label, Tag = c.Value });
        combo.SelectedIndex = 0;
        var box = new TextBox { Margin = new Thickness(0, 6, 0, 10), MinWidth = 280 };
        var okButton = new Button { Content = ok, IsDefault = true, Padding = new Thickness(12, 2, 12, 2), Margin = new Thickness(0, 0, 6, 0) };
        var cancelButton = new Button { Content = cancel, IsCancel = true, Padding = new Thickness(12, 2, 12, 2) };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
        buttons.Children.Add(okButton);
        buttons.Children.Add(cancelButton);
        var panel = new StackPanel { Margin = new Thickness(14) };
        panel.Children.Add(new TextBlock { Text = choiceLabel });
        panel.Children.Add(combo);
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
        if (win.ShowDialog() != true) return null;
        return (((ComboBoxItem)combo.SelectedItem).Tag as string ?? "flowchart", box.Text);
    }

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

    private static Window Dialog(Window owner, string title, UIElement content) => new()
    {
        Title = title, Owner = owner, Content = content, SizeToContent = SizeToContent.WidthAndHeight,
        ResizeMode = ResizeMode.NoResize, WindowStartupLocation = WindowStartupLocation.CenterOwner,
    };

    /// <summary>Read-only, selectable text (fallback when the clipboard is unavailable).</summary>
    public static void ShowText(Window owner, string title, string label, string text)
    {
        var box = new TextBox
        {
            Text = text, IsReadOnly = true, TextWrapping = TextWrapping.Wrap, FontFamily = new FontFamily("Cascadia Mono, Consolas"),
            Width = 520, MaxHeight = 320, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Margin = new Thickness(0, 6, 0, 10),
        };
        var close = new Button { Content = "OK", IsDefault = true, IsCancel = true, Padding = new Thickness(12, 2, 12, 2), HorizontalAlignment = HorizontalAlignment.Right };
        var panel = new StackPanel { Margin = new Thickness(14) };
        panel.Children.Add(new TextBlock { Text = label, TextWrapping = TextWrapping.Wrap, MaxWidth = 520 });
        panel.Children.Add(box);
        panel.Children.Add(close);
        var win = Dialog(owner, title, panel);
        box.Loaded += (_, _) => { box.Focus(); box.SelectAll(); };
        win.ShowDialog();
    }

    /// <summary>Update offer: returns "install", "page" or null.</summary>
    public static string? Update(Window owner, string title, string notes, string install, string later, string page)
    {
        string? choice = null;
        var notesBox = new TextBox
        {
            Text = notes, IsReadOnly = true, TextWrapping = TextWrapping.Wrap, Width = 520, MaxHeight = 300,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Margin = new Thickness(0, 8, 0, 10), BorderThickness = new Thickness(0),
        };
        var installButton = new Button { Content = install, IsDefault = true, Padding = new Thickness(12, 2, 12, 2), Margin = new Thickness(0, 0, 6, 0) };
        var pageButton = new Button { Content = page, Padding = new Thickness(12, 2, 12, 2), Margin = new Thickness(0, 0, 6, 0) };
        var laterButton = new Button { Content = later, IsCancel = true, Padding = new Thickness(12, 2, 12, 2) };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
        buttons.Children.Add(installButton);
        buttons.Children.Add(pageButton);
        buttons.Children.Add(laterButton);
        var panel = new StackPanel { Margin = new Thickness(14) };
        panel.Children.Add(new TextBlock { Text = title, FontWeight = FontWeights.SemiBold, FontSize = 14, TextWrapping = TextWrapping.Wrap, MaxWidth = 520 });
        panel.Children.Add(notesBox);
        panel.Children.Add(buttons);
        var win = Dialog(owner, "ODiagramador", panel);
        installButton.Click += (_, _) => { choice = "install"; win.Close(); };
        pageButton.Click += (_, _) => { choice = "page"; win.Close(); };
        win.ShowDialog();
        return choice;
    }

    /// <summary>Result of "copy MCP config": status, next step for that client and the text itself (selectable).</summary>
    public static void McpConfig(Window owner, string title, string status, string howto, string text, string copyAgain, Func<bool> copy)
    {
        var box = new TextBox
        {
            Text = text, IsReadOnly = true, TextWrapping = TextWrapping.Wrap, FontFamily = new FontFamily("Cascadia Mono, Consolas"),
            Width = 560, MaxHeight = 280, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Margin = new Thickness(0, 8, 0, 10),
        };
        var again = new Button { Content = copyAgain, Padding = new Thickness(12, 2, 12, 2), Margin = new Thickness(0, 0, 6, 0) };
        var close = new Button { Content = "OK", IsDefault = true, IsCancel = true, Padding = new Thickness(12, 2, 12, 2) };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right };
        buttons.Children.Add(again);
        buttons.Children.Add(close);
        var statusText = new TextBlock { Text = status, FontWeight = FontWeights.SemiBold, TextWrapping = TextWrapping.Wrap, MaxWidth = 560 };
        var panel = new StackPanel { Margin = new Thickness(14) };
        panel.Children.Add(statusText);
        panel.Children.Add(new TextBlock { Text = howto, TextWrapping = TextWrapping.Wrap, MaxWidth = 560, Margin = new Thickness(0, 6, 0, 0) });
        panel.Children.Add(box);
        panel.Children.Add(buttons);
        var win = Dialog(owner, "MCP · " + title, panel);
        again.Click += (_, _) =>
        {
            box.Focus();
            box.SelectAll();
            if (copy()) statusText.Text = "✓  " + status.TrimStart('✓', ' ');
        };
        win.ShowDialog();
    }
}
