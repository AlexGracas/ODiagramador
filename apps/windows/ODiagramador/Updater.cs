using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Net.Http;
using System.Reflection;
using System.Text.Json.Nodes;
using System.Windows;

namespace ODiagramador;

public record UpdateInfo(bool Available, string Current, string Latest, string PageUrl, string? AssetName, string? AssetUrl,
    long? AssetSize, string Notes);

public record CoreVersion(string Version, string Mermaid, string ReleasesApi, string ReleasesPage);

/// <summary>Checks GitHub for a newer release and replaces this installation with it.</summary>
public sealed class Updater
{
    private static readonly HttpClient Http = new() { Timeout = TimeSpan.FromMinutes(5) };

    public UpdateInfo? Available { get; private set; }

    public static string CurrentVersion
    {
        get
        {
            var v = Assembly.GetExecutingAssembly().GetName().Version;
            return v is null ? "0.0.0" : $"{v.Major}.{v.Minor}.{v.Build}";
        }
    }

    public async Task<UpdateInfo?> CheckAsync()
    {
        var version = Core.Call<CoreVersion>("version");
        using var req = new HttpRequestMessage(HttpMethod.Get, version.ReleasesApi);
        req.Headers.UserAgent.ParseAdd($"ODiagramador/{CurrentVersion}");
        req.Headers.Accept.ParseAdd("application/vnd.github+json");
        using var resp = await Http.SendAsync(req);
        resp.EnsureSuccessStatusCode();
        var json = await resp.Content.ReadAsStringAsync();
        var info = Core.CallNode<UpdateInfo>("update_evaluate", new JsonObject
        {
            ["release_json"] = json, ["current"] = CurrentVersion, ["platform"] = "windows",
        });
        Available = info.Available ? info : null;
        return info;
    }

    /// <summary>Downloads the package, then a PowerShell helper copies it over this folder once we exit and restarts the app.</summary>
    public async Task InstallAsync(Action<string> progress)
    {
        var info = Available ?? throw new InvalidOperationException("no update available");
        var installDir = AppContext.BaseDirectory.TrimEnd('\\');
        if (info.AssetUrl is null || !CanWrite(installDir))
            throw new UnauthorizedAccessException("manual");

        var work = Path.Combine(Path.GetTempPath(), "odg-update-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(work);
        var zip = Path.Combine(work, info.AssetName ?? "update.zip");
        await using (var src = await Http.GetStreamAsync(info.AssetUrl))
        await using (var dst = File.Create(zip))
        {
            await src.CopyToAsync(dst);
        }
        var extracted = Path.Combine(work, "pkg");
        ZipFile.ExtractToDirectory(zip, extracted);
        var payload = Directory.Exists(Path.Combine(extracted, "ODiagramador")) ? Path.Combine(extracted, "ODiagramador") : extracted;
        if (!File.Exists(Path.Combine(payload, "ODiagramador.exe")))
            throw new InvalidDataException("package has no ODiagramador.exe");

        var script = Path.Combine(work, "swap.ps1");
        File.WriteAllText(script, """
            param([int]$ProcId, [string]$Src, [string]$Dst)
            try { Wait-Process -Id $ProcId -Timeout 120 -ErrorAction SilentlyContinue } catch {}
            Start-Sleep -Milliseconds 500
            Copy-Item -Path (Join-Path $Src '*') -Destination $Dst -Recurse -Force
            Start-Process -FilePath (Join-Path $Dst 'ODiagramador.exe')
            """);
        var psi = new ProcessStartInfo("powershell.exe")
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            WindowStyle = ProcessWindowStyle.Hidden,
        };
        foreach (var a in new[] { "-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-File", script,
                     "-ProcId", Environment.ProcessId.ToString(), "-Src", payload, "-Dst", installDir })
            psi.ArgumentList.Add(a);
        Process.Start(psi);
        progress("restart");
        await Task.Delay(800);
        Application.Current.Shutdown();
        await Task.Delay(2000);
        Environment.Exit(0); // Shutdown can be blocked by a modal dialog; the helper is waiting for us
    }

    private static bool CanWrite(string dir)
    {
        try
        {
            var probe = Path.Combine(dir, ".odg-write-test");
            File.WriteAllText(probe, "");
            File.Delete(probe);
            return true;
        }
        catch
        {
            return false;
        }
    }
}
