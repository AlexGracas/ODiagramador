using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace ODiagramador;

/// <summary>Preferences: General (language, appearance, updates), Editor and diagram (fonts), MCP (clients, remote access).</summary>
public sealed class SettingsWindow : Window
{
    private readonly MainWindow _main;
    private readonly TabControl _tabs = new() { Margin = new Thickness(10) };
    private TextBlock _remoteStatus = new();
    private bool _loading;

    private Preferences Prefs => _main.Prefs;
    private string T(string key) => _main.T(key);
    private string T(string key, object args) => _main.T(key, args);

    public SettingsWindow(MainWindow main, string tab)
    {
        _main = main;
        Width = 720;
        SizeToContent = SizeToContent.Height;
        ResizeMode = ResizeMode.NoResize;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Content = _tabs;
        _main.Remote.Changed += OnRemoteChanged;
        Closed += (_, _) => _main.Remote.Changed -= OnRemoteChanged;
        Build();
        SelectTab(tab);
    }

    public void SelectTab(string tab) => _tabs.SelectedIndex = tab switch { "editor" => 1, "mcp" => 2, _ => 0 };

    /// <summary>Rebuilds every tab (also after a language change).</summary>
    private void Build()
    {
        _loading = true;
        var selected = Math.Max(0, _tabs.SelectedIndex);
        Title = T("ui-settings");
        _tabs.Items.Clear();
        _tabs.Items.Add(new TabItem { Header = T("ui-settings-general"), Content = Page(General()) });
        _tabs.Items.Add(new TabItem { Header = T("ui-settings-editor"), Content = Page(Editor()) });
        _tabs.Items.Add(new TabItem { Header = T("ui-settings-mcp"), Content = Page(Mcp()) });
        _tabs.SelectedIndex = selected;
        _loading = false;
    }

    private static UIElement Page(UIElement content) => new ScrollViewer
    {
        Content = content, MaxHeight = 620, VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Padding = new Thickness(14, 10, 14, 14),
    };

    // ------------------------------------------------------------------ General

    private UIElement General()
    {
        var panel = new StackPanel();
        var lang = Combo(new[] { ("en", "English"), ("pt-BR", "Português (Brasil)") }, _main.Lang);
        lang.SelectionChanged += (_, _) =>
        {
            if (_loading || Selected(lang) is not { } value) return;
            _main.SetLanguage(value);
            Build();
        };
        panel.Children.Add(Row(T("ui-language"), lang));

        var appearance = Combo(new[]
        {
            ("system", T("ui-appearance-system")), ("light", T("ui-appearance-light")), ("dark", T("ui-appearance-dark")),
        }, Prefs.Appearance);
        appearance.SelectionChanged += (_, _) =>
        {
            if (_loading || Selected(appearance) is not { } value) return;
            Prefs.Appearance = value;
            Prefs.Save();
            _main.ApplyEditorAppearance();
        };
        panel.Children.Add(Row(T("ui-appearance"), appearance));

        panel.Children.Add(Header(T("ui-updates")));
        var auto = Check(T("ui-auto-update"), Prefs.AutoUpdate, v => Prefs.AutoUpdate = v);
        var install = Check(T("ui-auto-install"), Prefs.AutoInstallUpdates, v => Prefs.AutoInstallUpdates = v);
        install.Margin = new Thickness(22, 4, 0, 4);
        install.IsEnabled = Prefs.AutoUpdate;
        auto.Checked += (_, _) => install.IsEnabled = true;
        auto.Unchecked += (_, _) => install.IsEnabled = false;
        panel.Children.Add(auto);
        panel.Children.Add(install);
        var check = new Button { Content = T("ui-check-updates"), Padding = new Thickness(12, 3, 12, 3) };
        check.Click += async (_, _) =>
        {
            check.IsEnabled = false;
            await _main.CheckForUpdatesAsync(userInitiated: true);
            check.IsEnabled = true;
        };
        var version = Muted(T("ui-version", new { version = Updater.CurrentVersion }));
        version.Margin = new Thickness(12, 0, 0, 0);
        var line = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 8, 0, 0) };
        line.Children.Add(check);
        line.Children.Add(version);
        panel.Children.Add(line);

        // diagnostics: where the log is, one click to open it
        panel.Children.Add(Header(T("ui-logs")));
        var logs = new Button { Content = T("ui-open-logs"), Padding = new Thickness(12, 3, 12, 3), HorizontalAlignment = HorizontalAlignment.Left };
        logs.Click += (_, _) =>
        {
            App.Log("open logs folder");
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("explorer.exe", $"/select,\"{App.LogPath}\"") { UseShellExecute = true });
        };
        panel.Children.Add(logs);
        var path = new TextBox { Text = App.LogPath, IsReadOnly = true, BorderThickness = new Thickness(0), Background = Brushes.Transparent, Margin = new Thickness(0, 6, 0, 0), FontSize = 12 };
        panel.Children.Add(path);
        return panel;
    }

    // ------------------------------------------------------------------ Editor and diagram

    private UIElement Editor()
    {
        var panel = new StackPanel();
        var families = Fonts.SystemFontFamilies.Select(f => f.Source).OrderBy(n => n, StringComparer.OrdinalIgnoreCase).ToList();

        var editorFont = new ComboBox { IsEditable = true, Width = 300, Text = Prefs.EditorFont, ItemsSource = families };
        editorFont.LostFocus += (_, _) => SetEditorFont(editorFont.Text);
        editorFont.SelectionChanged += (_, _) =>
        {
            if (!_loading && editorFont.SelectedItem is string f) SetEditorFont(f);
        };
        panel.Children.Add(Row(T("ui-editor-font"), editorFont));

        var size = new ComboBox { Width = 90, IsEditable = true, ItemsSource = new[] { 10, 11, 12, 13, 14, 15, 16, 18, 20, 24 }, Text = Prefs.EditorFontSize.ToString() };
        void SetSize()
        {
            if (_loading || !double.TryParse(size.Text, out var v) || v is < 8 or > 40) return;
            Prefs.EditorFontSize = v;
            Prefs.Save();
            _main.ApplyEditorAppearance();
        }
        size.SelectionChanged += (_, _) => Dispatcher.BeginInvoke(SetSize);
        size.LostFocus += (_, _) => SetSize();
        panel.Children.Add(Row(T("ui-editor-font-size"), size));

        var diagram = new ComboBox { IsEditable = true, Width = 300 };
        diagram.Items.Add(T("ui-font-default"));
        foreach (var f in families) diagram.Items.Add(f);
        diagram.Text = string.IsNullOrEmpty(_main.DiagramFont) ? T("ui-font-default") : _main.DiagramFont;
        void SetDiagram()
        {
            if (_loading) return;
            var value = diagram.Text == T("ui-font-default") ? "" : diagram.Text.Trim();
            if (value != _main.DiagramFont) _main.DiagramFont = value;
        }
        diagram.SelectionChanged += (_, _) => Dispatcher.BeginInvoke(SetDiagram);
        diagram.LostFocus += (_, _) => SetDiagram();
        panel.Children.Add(Row(T("ui-diagram-font"), diagram));
        panel.Children.Add(Muted(T("ui-diagram-font-hint")));
        return panel;
    }

    private void SetEditorFont(string family)
    {
        family = family.Trim();
        if (_loading || family.Length == 0 || family == Prefs.EditorFont) return;
        Prefs.EditorFont = family;
        Prefs.Save();
        _main.ApplyEditorAppearance();
    }

    // ------------------------------------------------------------------ MCP

    private UIElement Mcp()
    {
        var panel = new StackPanel();
        panel.Children.Add(Header(T("ui-mcp-local")));
        panel.Children.Add(ClientButtons(remote: false));
        panel.Children.Add(Muted(T("ui-mcp-local-hint")));

        panel.Children.Add(Header(T("ui-mcp-remote")));
        var enable = Check(T("ui-mcp-remote-enable"), Prefs.McpRemote, v => Prefs.McpRemote = v);
        panel.Children.Add(enable);

        var details = new StackPanel { Margin = new Thickness(22, 4, 0, 0), IsEnabled = Prefs.McpRemote };
        enable.Checked += (_, _) => { details.IsEnabled = true; _main.Remote.Restart(); };
        enable.Unchecked += (_, _) => { details.IsEnabled = false; _main.Remote.Restart(); };

        var port = new TextBox { Width = 90, Text = Prefs.McpPort.ToString() };
        port.LostFocus += (_, _) =>
        {
            if (int.TryParse(port.Text, out var p) && p is > 0 and < 65536 && p != Prefs.McpPort)
            {
                Prefs.McpPort = p;
                Prefs.Save();
                _main.Remote.Restart();
            }
            else port.Text = Prefs.McpPort.ToString();
        };
        details.Children.Add(Row(T("ui-mcp-port"), port));

        var scope = Combo(new[] { ("local", T("ui-mcp-scope-local")), ("lan", T("ui-mcp-scope-lan")) }, Prefs.McpLan ? "lan" : "local");
        scope.SelectionChanged += (_, _) =>
        {
            if (_loading) return;
            Prefs.McpLan = Selected(scope) == "lan";
            Prefs.Save();
            _main.Remote.Restart();
        };
        details.Children.Add(Row(T("ui-mcp-scope"), scope));

        var token = new TextBox { Text = Prefs.McpToken, IsReadOnly = true, Width = 360, FontFamily = new FontFamily("Cascadia Mono, Consolas") };
        var regenerate = new Button { Content = T("ui-mcp-regenerate"), Padding = new Thickness(10, 2, 10, 2), Margin = new Thickness(6, 0, 0, 0) };
        regenerate.Click += (_, _) =>
        {
            Prefs.McpToken = RemoteMcp.NewToken();
            Prefs.Save();
            token.Text = Prefs.McpToken;
            _main.Remote.Restart();
        };
        var tokenRow = new StackPanel { Orientation = Orientation.Horizontal };
        tokenRow.Children.Add(token);
        tokenRow.Children.Add(regenerate);
        details.Children.Add(Row(T("ui-mcp-token"), tokenRow));

        _remoteStatus = new TextBlock { Margin = new Thickness(0, 6, 0, 6), TextWrapping = TextWrapping.Wrap };
        UpdateRemoteStatus();
        details.Children.Add(_remoteStatus);
        details.Children.Add(new TextBlock { Text = T("ui-mcp-copy-remote"), Margin = new Thickness(0, 4, 0, 4) });
        details.Children.Add(ClientButtons(remote: true));
        var warning = Muted("⚠  " + T("ui-mcp-remote-warning"));
        warning.Margin = new Thickness(0, 8, 0, 0);
        details.Children.Add(warning);
        panel.Children.Add(details);
        return panel;
    }

    private UIElement ClientButtons(bool remote)
    {
        var wrap = new WrapPanel { Margin = new Thickness(0, 4, 0, 6) };
        foreach (var (id, name) in MainWindow.McpClients.Append(("json", T("ui-mcp-other"))))
        {
            var content = new StackPanel { Orientation = Orientation.Horizontal };
            content.Children.Add(BrandIcons.ForClient(id));
            content.Children.Add(new TextBlock { Text = name, Margin = new Thickness(6, 0, 0, 0), VerticalAlignment = VerticalAlignment.Center });
            var button = new Button { Content = content, Padding = new Thickness(10, 4, 10, 4), Margin = new Thickness(0, 0, 6, 6) };
            button.Click += (_, _) => _main.CopyMcp(id, remote, this);
            wrap.Children.Add(button);
        }
        return wrap;
    }

    private void OnRemoteChanged() => Dispatcher.BeginInvoke(UpdateRemoteStatus);

    private void UpdateRemoteStatus()
    {
        var remote = _main.Remote;
        (_remoteStatus.Text, _remoteStatus.Foreground) = remote.Status switch
        {
            "running" => ("●  " + T("ui-mcp-running", new { url = remote.Url }), (Brush)Brushes.SeaGreen),
            "failed" => ("●  " + T("ui-mcp-error", new { detail = remote.Detail ?? "" }), Brushes.IndianRed),
            _ => ("○  " + T("ui-mcp-stopped"), Brushes.Gray),
        };
    }

    // ------------------------------------------------------------------ helpers

    private static FrameworkElement Row(string label, FrameworkElement control)
    {
        var grid = new Grid { Margin = new Thickness(0, 4, 0, 4) };
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(210) });
        grid.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        var text = new TextBlock { Text = label, VerticalAlignment = VerticalAlignment.Center, TextWrapping = TextWrapping.Wrap };
        control.HorizontalAlignment = HorizontalAlignment.Left;
        Grid.SetColumn(control, 1);
        grid.Children.Add(text);
        grid.Children.Add(control);
        return grid;
    }

    private static TextBlock Header(string text) => new()
    {
        Text = text, FontWeight = FontWeights.SemiBold, FontSize = 14, Margin = new Thickness(0, 14, 0, 4),
    };

    private static TextBlock Muted(string text)
    {
        var block = new TextBlock { Text = text, TextWrapping = TextWrapping.Wrap, FontSize = 12, Opacity = 0.75, Margin = new Thickness(0, 2, 0, 2) };
        return block;
    }

    private CheckBox Check(string label, bool value, Action<bool> set)
    {
        var box = new CheckBox { Content = label, IsChecked = value, Margin = new Thickness(0, 4, 0, 4) };
        box.Checked += (_, _) => { set(true); Prefs.Save(); };
        box.Unchecked += (_, _) => { set(false); Prefs.Save(); };
        return box;
    }

    private static ComboBox Combo((string Value, string Label)[] items, string selected)
    {
        var combo = new ComboBox { MinWidth = 300 };
        foreach (var (value, label) in items)
        {
            combo.Items.Add(new ComboBoxItem { Content = label, Tag = value });
            if (value == selected) combo.SelectedIndex = combo.Items.Count - 1;
        }
        if (combo.SelectedIndex < 0) combo.SelectedIndex = 0;
        return combo;
    }

    private static string? Selected(ComboBox combo) => (combo.SelectedItem as ComboBoxItem)?.Tag as string;
}
