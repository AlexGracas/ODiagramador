using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace ODiagramador;

/// <summary>Wrapper over the Rust core: odg_invoke(method, json) → {"ok":..,"result"|"error"}.</summary>
public static class Core
{
    [DllImport("odg_ffi", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr odg_invoke(byte[] method, byte[] argsJson);

    [DllImport("odg_ffi", CallingConvention = CallingConvention.Cdecl)]
    private static extern void odg_free(IntPtr s);

    public static readonly JsonSerializerOptions Json = new()
    {
        PropertyNamingPolicy = new SnakeCase(),
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    private static byte[] Utf8Z(string s) => Encoding.UTF8.GetBytes(s + "\0");

    private static JsonNode? Raw(string method, object? args)
    {
        var json = args is null ? "{}" : JsonSerializer.Serialize(args, Json);
        var ptr = odg_invoke(Utf8Z(method), Utf8Z(json));
        if (ptr == IntPtr.Zero) throw new InvalidOperationException("odg_invoke returned NULL");
        try
        {
            var text = Marshal.PtrToStringUTF8(ptr) ?? "{}";
            var env = JsonNode.Parse(text)!;
            if (env["ok"]?.GetValue<bool>() != true)
                throw new InvalidOperationException(env["error"]?.GetValue<string>() ?? "unknown error");
            return env["result"];
        }
        finally
        {
            odg_free(ptr);
        }
    }

    public static T Call<T>(string method, object? args = null) =>
        Raw(method, args).Deserialize<T>(Json) ?? throw new InvalidOperationException($"{method} returned nothing");

    public static T? CallOrDefault<T>(string method, object? args = null)
    {
        try { return Raw(method, args).Deserialize<T>(Json); }
        catch { return default; }
    }

    public static void Run(string method, object? args = null) => Raw(method, args);

    /// <summary>Calls with a pre-built JSON object (used to forward the render-kit result untouched).</summary>
    public static T CallNode<T>(string method, JsonObject args)
    {
        var ptr = odg_invoke(Utf8Z(method), Utf8Z(args.ToJsonString()));
        try
        {
            var env = JsonNode.Parse(Marshal.PtrToStringUTF8(ptr) ?? "{}")!;
            if (env["ok"]?.GetValue<bool>() != true)
                throw new InvalidOperationException(env["error"]?.GetValue<string>() ?? "unknown error");
            return env["result"].Deserialize<T>(Json)!;
        }
        finally
        {
            odg_free(ptr);
        }
    }

    private sealed class SnakeCase : JsonNamingPolicy
    {
        public override string ConvertName(string name)
        {
            var sb = new StringBuilder();
            for (var i = 0; i < name.Length; i++)
            {
                var c = name[i];
                if (char.IsUpper(c))
                {
                    if (i > 0) sb.Append('_');
                    sb.Append(char.ToLowerInvariant(c));
                }
                else sb.Append(c);
            }
            return sb.ToString();
        }
    }
}

// ---- models (mirror odg-core JSON)

public record BBox(double X, double Y, double W, double H);

public record Problem(int Index, string Severity, string Code, string Message, string Hint, int? Line, int? Column,
    string? ElementId, List<string>? Related, BBox? Bbox)
{
    public string Title => $"{Index}. {Message}";
    public string Location => Line is int l ? $"{Code} · {l}" : Code;
    public string Glyph => Severity switch { "error" => "⛔", "warning" => "⚠", _ => "ℹ" };
}

public record Stats(int Nodes, int Edges, int Groups, double Width, double Height);

public record Report(string? Path, string? DiagramType, bool Valid, int ErrorCount, int WarningCount, int InfoCount,
    string Summary, List<Problem> Problems, Stats Stats);

public record DiagramStatus(bool Valid, int Errors, int Warnings, bool Stale, string Summary);

public record DiagramInfo(string Path, string Name, string Engine, long Size, string Modified, DiagramStatus? Status, int OpenFeedback,
    string? Kind = null, string? KindLabel = null)
{
    public string Glyph => Status is null ? "○" : !Status.Valid ? "⛔" : Status.Warnings > 0 ? "⚠" : "✔";
    public string Badge => OpenFeedback > 0 ? $"💬 {OpenFeedback}" : "";
    /// Folder inside the workspace ("" = root).
    public string Folder => Path.Contains('/') ? Path[..Path.LastIndexOf('/')] : "";
    public string FolderText => Folder.Length > 0 ? Folder + "/" : "";
    /// Group titles, filled by the window (they depend on the UI language).
    public string KindGroup { get; set; } = "";
    public string FolderGroup { get; set; } = "";
    public System.Windows.Visibility KindVisibility => KindLabel is null ? System.Windows.Visibility.Collapsed : System.Windows.Visibility.Visible;
    /// Colour of the type tag, by family of diagram.
    public string KindColor => Kind switch
    {
        "flowchart" or "block" or "architecture" or "c4" or "swimlane" or "agentflow" or "requirement" => "#1A73E8",
        "sequence" or "zenuml" or "state" or "usecase" or "eventmodeling" => "#8250DF",
        "class" or "er" => "#0E8A8A",
        "gantt" or "timeline" or "journey" or "kanban" or "gitgraph" => "#D9730D",
        "mindmap" or "treeview" or "ishikawa" => "#C2185B",
        _ => "#1E8E3E",
    };
    public string KindBackground => "#22" + KindColor[1..];
}

public record Feedback(string Id, string Path, string? ElementId, int? Line, string Text, string? Context, string Author, string CreatedAt,
    bool Resolved, string? Resolution)
{
    public string Glyph => Resolved ? "✔" : "💬";
    public string Meta => Author + (ElementId is null ? "" : $" · {ElementId}") + (Resolution is null ? "" : $" · {Resolution}");
}

public record ActivityEvent(string Ts, string Actor, string Action, string Path, bool? Valid, int? Errors, int? Warnings,
    string? Note, string? Client)
{
    public string Glyph => Actor switch { "agent" => "✦", "human" => "👤", _ => "⌘" };
    public string Line => $"{Action,-9} {Path}{(Valid is bool v ? (v ? " ✓" : " ✗") : "")}  {Client}";
    public string When => Ts.Replace('T', ' ')[..Math.Min(19, Ts.Length)];
}

public record DiagramKind(string Kind, string Label, bool Graph);

public record WorkspaceConfig(string? Lang, string? Theme, string? Font);

public record McpConfig(string Json, string ClaudeCode, string Codex, string? RemoteJson, string? RemoteClaudeCode, string? RemoteCodex);
