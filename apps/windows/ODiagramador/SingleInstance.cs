using System.IO;
using System.IO.Pipes;
using System.Windows;

namespace ODiagramador;

/// <summary>
/// One ODiagramador per user session. Agents open the viewer through odiagramador:// links (show_diagram,
/// highlight_elements, ask_user…), and every link used to start another process — another window. A second
/// launch now hands its arguments to the running instance over a named pipe and exits.
/// </summary>
public static class SingleInstance
{
    private static Mutex? _mutex;
    private static string PipeName => "ODiagramador-" + Environment.UserName;

    public static bool TryBecomePrimary()
    {
        _mutex = new Mutex(true, @"Local\ODiagramador-single-instance", out var created);
        return created;
    }

    /// <summary>Sends the command line to the running instance; false if it did not answer.</summary>
    public static bool Forward(string[] args)
    {
        try
        {
            using var client = new NamedPipeClientStream(".", PipeName, PipeDirection.Out, PipeOptions.CurrentUserOnly);
            client.Connect(3000);
            using var writer = new StreamWriter(client);
            writer.Write(string.Join("\n", args));
            writer.Flush();
            return true;
        }
        catch (Exception ex)
        {
            App.Log("single instance: forward failed: " + ex.Message);
            return false;
        }
    }

    /// <summary>Receives the command lines of later launches and runs <paramref name="onArgs"/> on the UI thread.</summary>
    public static void Listen(Action<string[]> onArgs)
    {
        var thread = new Thread(() =>
        {
            while (true)
            {
                try
                {
                    using var server = new NamedPipeServerStream(PipeName, PipeDirection.In, 1, PipeTransmissionMode.Byte, PipeOptions.CurrentUserOnly);
                    server.WaitForConnection();
                    using var reader = new StreamReader(server);
                    var args = reader.ReadToEnd().Split('\n', StringSplitOptions.RemoveEmptyEntries);
                    App.Log($"single instance: request from another launch: {string.Join(" ", args)}");
                    Application.Current.Dispatcher.BeginInvoke(() => onArgs(args));
                }
                catch (Exception ex)
                {
                    App.Log("single instance: listener: " + ex.Message);
                    Thread.Sleep(500);
                }
            }
        })
        { IsBackground = true, Name = "single-instance" };
        thread.Start();
    }
}
