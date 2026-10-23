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

    /// <summary>Suffix of the files an update moved aside; removed on the next start.</summary>
    public const string AsideMarker = ".odg-old-";

    /// <summary>
    /// Downloads the package and swaps it in from inside the app: Windows lets a running .exe/.dll be renamed,
    /// so every old file is moved aside and the new one copied in its place. Any failure rolls everything back,
    /// the new version is verified, then the app restarts. Each step goes to the log.
    /// </summary>
    public async Task InstallAsync(Action<string> progress)
    {
        var info = Available ?? throw new InvalidOperationException("no update available");
        var installDir = AppContext.BaseDirectory.TrimEnd('\\');
        if (info.AssetUrl is null || !CanWrite(installDir))
        {
            App.Log($"update: manual install needed (asset={info.AssetUrl ?? "none"}, writable={CanWrite(installDir)}) in {installDir}");
            throw new UnauthorizedAccessException("manual");
        }

        var work = Path.Combine(Path.GetTempPath(), "odg-update-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(work);
        var zip = Path.Combine(work, info.AssetName ?? "update.zip");
        App.Log($"update: {info.Current} -> {info.Latest}, downloading {info.AssetUrl}");
        await using (var src = await Http.GetStreamAsync(info.AssetUrl))
        await using (var dst = File.Create(zip))
        {
            await src.CopyToAsync(dst);
        }
        var extracted = Path.Combine(work, "pkg");
        await Task.Run(() => ZipFile.ExtractToDirectory(zip, extracted));
        var payload = Directory.Exists(Path.Combine(extracted, "ODiagramador")) ? Path.Combine(extracted, "ODiagramador") : extracted;
        if (!File.Exists(Path.Combine(payload, "ODiagramador.exe")))
            throw new InvalidDataException("package has no ODiagramador.exe");
        var expected = VersionOf(Path.Combine(payload, "ODiagramador.dll"));
        App.Log($"update: package {expected} extracted to {payload}; installing into {installDir}");

        var count = await Task.Run(() => SwapFiles(payload, installDir, AsideMarker + DateTime.Now.Ticks));
        var installed = VersionOf(Path.Combine(installDir, "ODiagramador.dll"));
        App.Log($"update: {count} files replaced; installed version {installed}");
        if (installed != expected) throw new IOException($"the files were copied but the installed version is {installed}, expected {expected}");
        try { Directory.Delete(work, true); } catch { /* temp folder, best effort */ }

        progress("restart");
        var psi = new ProcessStartInfo(Path.Combine(installDir, "ODiagramador.exe")) { UseShellExecute = false, WorkingDirectory = installDir };
        psi.ArgumentList.Add("--after-update");
        psi.ArgumentList.Add(Environment.ProcessId.ToString());
        Process.Start(psi);
        await Task.Delay(500);
        Application.Current.Shutdown();
        await Task.Delay(2000);
        Environment.Exit(0); // Shutdown can be blocked by a modal dialog
    }

    /// <summary>Copies every file of <paramref name="payload"/> over <paramref name="installDir"/>, moving existing
    /// files aside first (allowed even while they run). On error, restores the previous files and rethrows.</summary>
    public static int SwapFiles(string payload, string installDir, string asideSuffix)
    {
        var moved = new List<(string Dest, string Aside)>();
        var added = new List<string>();
        try
        {
            foreach (var src in Directory.EnumerateFiles(payload, "*", SearchOption.AllDirectories))
            {
                var dest = Path.Combine(installDir, Path.GetRelativePath(payload, src));
                Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
                if (File.Exists(dest))
                {
                    var aside = dest + asideSuffix;
                    File.Move(dest, aside);
                    moved.Add((dest, aside));
                }
                else
                {
                    added.Add(dest);
                }
                File.Copy(src, dest);
            }
            return moved.Count + added.Count;
        }
        catch (Exception ex)
        {
            App.Log($"update: swap failed after {moved.Count + added.Count} files, rolling back: {ex}");
            foreach (var dest in added)
            {
                try { File.Delete(dest); } catch (Exception e) { App.Log("rollback: " + e.Message); }
            }
            foreach (var (dest, aside) in Enumerable.Reverse(moved))
            {
                try
                {
                    if (File.Exists(dest)) File.Delete(dest);
                    File.Move(aside, dest);
                }
                catch (Exception e)
                {
                    App.Log("rollback: " + e.Message);
                }
            }
            throw;
        }
    }

    /// <summary>Removes files a previous update moved aside (they were still in use then).</summary>
    public static void CleanLeftovers()
    {
        try
        {
            var left = Directory.EnumerateFiles(AppContext.BaseDirectory, "*" + AsideMarker + "*", SearchOption.AllDirectories).ToList();
            var removed = 0;
            foreach (var f in left)
            {
                try
                {
                    File.Delete(f);
                    removed++;
                }
                catch
                {
                    // still in use (another instance / an agent's odg.exe): next start
                }
            }
            if (left.Count > 0) App.Log($"update: removed {removed} of {left.Count} old files");
        }
        catch (Exception ex)
        {
            App.Log("cleanup: " + ex.Message);
        }
    }

    private static string VersionOf(string file)
    {
        var v = FileVersionInfo.GetVersionInfo(file);
        return $"{v.FileMajorPart}.{v.FileMinorPart}.{v.FileBuildPart}";
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
