using System.IO;
using System.Text.Json;

namespace ODiagramador;

/// <summary>Per-user settings (%AppData%\ODiagramador\settings.json).</summary>
public sealed class Preferences
{
    public string? Language { get; set; }
    /// <summary>system | light | dark</summary>
    public string Appearance { get; set; } = "system";
    public string EditorFont { get; set; } = "Cascadia Mono";
    public double EditorFontSize { get; set; } = 13;
    public bool AutoUpdate { get; set; } = true;
    public bool AutoInstallUpdates { get; set; }
    public DateTime LastUpdateCheck { get; set; }
    public bool McpRemote { get; set; }
    public int McpPort { get; set; } = 7410;
    public bool McpLan { get; set; }
    public string McpToken { get; set; } = RemoteMcp.NewToken();
    public string? LastWorkspace { get; set; }

    private static readonly string Dir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "ODiagramador");
    private static readonly string FilePath = Path.Combine(Dir, "settings.json");

    public static Preferences Load()
    {
        try
        {
            if (File.Exists(FilePath)) return JsonSerializer.Deserialize<Preferences>(File.ReadAllText(FilePath)) ?? new();
            // migrate the workspace remembered by 0.1.x
            var old = Path.Combine(Dir, "last-workspace");
            return new Preferences { LastWorkspace = File.Exists(old) ? File.ReadAllText(old).Trim() : null };
        }
        catch
        {
            return new Preferences();
        }
    }

    public void Save()
    {
        try
        {
            Directory.CreateDirectory(Dir);
            File.WriteAllText(FilePath, JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true }));
        }
        catch
        {
            // settings are a convenience; never crash for them
        }
    }

    /// <summary>Dark mode in effect (system setting resolved through the registry).</summary>
    public bool IsDark
    {
        get
        {
            if (Appearance == "dark") return true;
            if (Appearance == "light") return false;
            try
            {
                using var key = Microsoft.Win32.Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
                return key?.GetValue("AppsUseLightTheme") is int v && v == 0;
            }
            catch
            {
                return false;
            }
        }
    }
}
