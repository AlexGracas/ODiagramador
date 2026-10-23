using System.Diagnostics;
using System.IO;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Security.Cryptography;

namespace ODiagramador;

/// <summary>
/// Runs the bundled `odg mcp --http` so agents on other machines can reach this workspace.
/// Off by default; LAN access always requires a bearer token (odg refuses otherwise).
/// </summary>
public sealed class RemoteMcp
{
    private readonly Preferences _prefs;
    private Process? _process;
    private string? _workspace;
    private string _odg = "";

    public string Status { get; private set; } = "stopped";
    public string? Detail { get; private set; }
    public event Action? Changed;

    public RemoteMcp(Preferences prefs) => _prefs = prefs;

    public static string NewToken() => Convert.ToHexString(RandomNumberGenerator.GetBytes(24)).ToLowerInvariant();

    public string Url
    {
        get
        {
            var host = _prefs.McpLan ? LanAddresses().FirstOrDefault() ?? "127.0.0.1" : "127.0.0.1";
            return $"http://{host}:{_prefs.McpPort}/mcp";
        }
    }

    public void Configure(string? workspace, string odgPath)
    {
        _workspace = workspace;
        _odg = odgPath;
        Restart();
    }

    public void Restart()
    {
        Stop();
        if (!_prefs.McpRemote || _workspace is null || !File.Exists(_odg) || _prefs.McpPort is < 1 or > 65535) return;
        var psi = new ProcessStartInfo(_odg)
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardError = true,
        };
        foreach (var a in new[] { "mcp", "--workspace", _workspace, "--http", $"{(_prefs.McpLan ? "0.0.0.0" : "127.0.0.1")}:{_prefs.McpPort}" })
            psi.ArgumentList.Add(a);
        psi.Environment["ODG_MCP_TOKEN"] = _prefs.McpToken;
        try
        {
            var p = new Process { StartInfo = psi, EnableRaisingEvents = true };
            var lastLine = "";
            p.ErrorDataReceived += (_, e) => { if (!string.IsNullOrWhiteSpace(e.Data)) lastLine = e.Data; };
            p.Exited += (_, _) =>
            {
                if (!ReferenceEquals(_process, p)) return;
                _process = null;
                Status = "failed";
                Detail = lastLine.Replace("odg: ", "");
                App.Log($"remote MCP exited (code {p.ExitCode}): {lastLine}");
                Changed?.Invoke();
            };
            p.Start();
            p.BeginErrorReadLine();
            App.Log($"remote MCP started: pid {p.Id} on {(_prefs.McpLan ? "0.0.0.0" : "127.0.0.1")}:{_prefs.McpPort}");
            _process = p;
            Status = "running";
            Detail = Url;
        }
        catch (Exception ex)
        {
            App.Log("remote MCP failed to start: " + ex);
            Status = "failed";
            Detail = ex.Message;
        }
        Changed?.Invoke();
    }

    public void Stop()
    {
        var p = _process;
        _process = null;
        if (p is not null)
        {
            try { if (!p.HasExited) p.Kill(); } catch { /* already gone */ }
        }
        Status = "stopped";
        Detail = null;
        Changed?.Invoke();
    }

    public static IEnumerable<string> LanAddresses() =>
        NetworkInterface.GetAllNetworkInterfaces()
            .Where(n => n.OperationalStatus == OperationalStatus.Up && n.NetworkInterfaceType != NetworkInterfaceType.Loopback)
            .SelectMany(n => n.GetIPProperties().UnicastAddresses)
            .Where(a => a.Address.AddressFamily == AddressFamily.InterNetwork)
            .Select(a => a.Address.ToString());
}
