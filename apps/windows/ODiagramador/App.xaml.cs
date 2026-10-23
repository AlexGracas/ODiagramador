using System.Windows;
using Microsoft.Win32;

namespace ODiagramador;

public partial class App : Application
{
    /// <summary>Command line: [--workspace DIR] [odiagramador://open?workspace=…&amp;path=…]</summary>
    public static string[] Args { get; private set; } = Array.Empty<string>();

    protected override void OnStartup(StartupEventArgs e)
    {
        Args = e.Args;
        RegisterUrlScheme();
        base.OnStartup(e);
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
