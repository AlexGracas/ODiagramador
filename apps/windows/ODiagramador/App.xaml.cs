using System.Windows;
using Microsoft.Win32;

namespace ODiagramador;

public partial class App : Application
{
    /// <summary>Command line: [--workspace DIR] [odiagramador://open?workspace=…&amp;path=…]</summary>
    public static string[] Args { get; private set; } = Array.Empty<string>();

    public static Preferences Prefs { get; } = Preferences.Load();

    protected override void OnStartup(StartupEventArgs e)
    {
        Args = e.Args;
        Log($"start {Updater.CurrentVersion} on {Environment.OSVersion}");
        DispatcherUnhandledException += (_, ex) =>
        {
            Log("unhandled: " + ex.Exception);
            MessageBox.Show(ex.Exception.Message + "\n\n" + LogPath, "ODiagramador", MessageBoxButton.OK, MessageBoxImage.Error);
            ex.Handled = true;
        };
        ApplyTheme(Prefs);
        RegisterUrlScheme();
        base.OnStartup(e);
    }

    /// <summary>Diagnostics for problems we cannot reproduce: %LocalAppData%\ODiagramador\odiagramador.log</summary>
    public static readonly string LogPath = System.IO.Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ODiagramador", "odiagramador.log");

    public static void Log(string line)
    {
        try
        {
            System.IO.Directory.CreateDirectory(System.IO.Path.GetDirectoryName(LogPath)!);
            var info = new System.IO.FileInfo(LogPath);
            if (info.Exists && info.Length > 512 * 1024) info.Delete();
            System.IO.File.AppendAllText(LogPath, $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} {line}{Environment.NewLine}");
        }
        catch
        {
            // logging must never break the app
        }
    }

    /// <summary>Fluent theme (light/dark/system) on .NET 9+; the classic theme otherwise.</summary>
    public static void ApplyTheme(Preferences prefs)
    {
#if NET9_0_OR_GREATER
#pragma warning disable WPF0001 // ThemeMode is marked experimental
        var mode = prefs.Appearance switch { "dark" => ThemeMode.Dark, "light" => ThemeMode.Light, _ => ThemeMode.System };
        if (Current.ThemeMode != mode) Current.ThemeMode = mode;
#pragma warning restore WPF0001
#endif
    }

    /// <summary>Registers odiagramador:// for the current user so the MCP tool open_in_viewer can reach the app.</summary>
    private static void RegisterUrlScheme()
    {
        try
        {
            var exe = Environment.ProcessPath;
            if (exe is null) return;
            using var key = Registry.CurrentUser.CreateSubKey(@"Software\Classes\odiagramador");
            key.SetValue("", "URL:ODiagramador");
            key.SetValue("URL Protocol", "");
            using var cmd = key.CreateSubKey(@"shell\open\command");
            cmd.SetValue("", $"\"{exe}\" \"%1\"");
        }
        catch
        {
            // not fatal: only open_in_viewer depends on it
        }
    }
}
