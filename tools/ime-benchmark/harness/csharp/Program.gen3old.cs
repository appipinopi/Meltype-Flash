// SPDX-License-Identifier: GPL-3.0-or-later
// 1万文コアベンチ用ハーネス 世代3・旧変換器インターフェース (v1.7 系の過渡期向け)
//
//   使い方: CoreBench <cases.jsonl> <out.jsonl> [mozcHelperPath|-] [profileDir]
//   ImeHost + SessionParts の骨格は v1.7 と同じで、変換器の IKanjiConverter だけが旧世代
//   (Convert(string) / ConvertClauses(string, string?) を Composition 側に持つ) の版向け。
//   精度の比較は Layer A/B ともこのハーネスで行い、使ったハーネス名を結果 JSON に残す。
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Meltype.Composition;
using Meltype.Config;
using Meltype.Detection;
using Meltype.Ime;

internal static class Program
{
    private static int Main(string[] args)
    {
        CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
        if (args.Length < 2)
        {
            Console.Error.WriteLine("usage: CoreBench <cases.jsonl> <out.jsonl> [mozcHelper|-] [profileDir]");
            return 2;
        }
        var helper = args.Length > 2 ? args[2] : "-";
        var profile = args.Length > 3 ? args[3] : null;
        BenchMozc? mozc = helper == "-" ? null : new BenchMozc(helper, profile);
        try
        {
            IKanjiConverter converter = mozc is null ? new NullConverter() : mozc;
            var host = CreateHost(converter);
            using var output = new StreamWriter(args[1], false, new UTF8Encoding(false));
            var index = 0;
            foreach (var line in File.ReadLines(args[0]))
            {
                if (string.IsNullOrWhiteSpace(line)) continue;
                using var document = JsonDocument.Parse(line);
                var root = document.RootElement;
                var id = root.GetProperty("id").GetInt32();
                var typed = root.GetProperty("typed").GetString() ?? "";
                var watch = Stopwatch.StartNew();
                var (enter, space) = Run(host, typed, "case-" + id);
                watch.Stop();
                output.WriteLine(JsonSerializer.Serialize(new Dictionary<string, object>
                {
                    ["id"] = id,
                    ["enter"] = enter,
                    ["space"] = space,
                    ["ms"] = Math.Round(watch.Elapsed.TotalMilliseconds, 3),
                }));
                if (++index % 500 == 0) Console.Error.WriteLine($"cases={index}");
            }
        }
        finally
        {
            mozc?.Dispose();
        }
        return 0;
    }

    private static ImeHost CreateHost(IKanjiConverter converter)
    {
        var detector = CompositionDetector.CreateDefault();
        var services = new CompositionServices();
        MeltypeRuntime.Connect(detector, services);
        var settings = new Settings().Normalize();
        return new ImeHost(new BenchEnvironment(), new SessionParts(detector, converter, services), () => settings);
    }

    private static (string Enter, string Space) Run(ImeHost host, string typed, string instance)
    {
        var committed = new StringBuilder();
        foreach (var c in typed)
        {
            var result = host.Handle(Key(instance, VirtualKey(c), c));
            Apply(committed, result.Result.Commits);
        }
        var spaceResult = host.Handle(Key(instance, 0x20, ' '));
        var spaceBuffer = new StringBuilder(committed.ToString());
        Apply(spaceBuffer, spaceResult.Result.Commits);
        var space = spaceResult.Result.View?.Text ?? spaceBuffer.ToString();
        var enterResult = host.Handle(Key(instance, 0x0D, '\0'));
        var enterBuffer = new StringBuilder(spaceBuffer.ToString());
        Apply(enterBuffer, enterResult.Result.Commits);
        host.Handle(new ImeEvent { Version = ImeHost.Version, Type = ImeEventType.InstanceDestroyed, Instance = instance });
        return (enterBuffer.ToString(), space);
    }

    private static ImeEvent Key(string instance, int vk, char ch) => new()
    {
        Version = ImeHost.Version,
        Type = ImeEventType.KeyPressed,
        Instance = instance,
        Key = new ImeKey { Vk = vk, Ch = ch },
    };

    private static int VirtualKey(char c) =>
        c == ' ' ? 0x20 : char.IsAsciiLetter(c) ? char.ToUpperInvariant(c) : 0;

    private static void Apply(StringBuilder buffer, IReadOnlyList<TextEdit> edits)
    {
        foreach (var edit in edits)
        {
            if (edit.DeleteBefore > 0) buffer.Length = Math.Max(0, buffer.Length - edit.DeleteBefore);
            buffer.Append(edit.Text);
        }
    }
}

internal sealed class BenchEnvironment : IImeEnvironment
{
    public InputMode? InputMethod => null;
    public SnippetStore Snippets { get; } = new(null);
    public string? ClipboardText => null;
    public string? OtherTipVersion => null;
}

/// <summary>変換器なしのコア (Layer A)。</summary>
internal sealed class NullConverter : IKanjiConverter
{
    public string? Convert(string hiragana) => null;
    public IReadOnlyList<ConversionClause>? ConvertClauses(string hiragana, string? context = null) => null;
}

/// <summary>同梱の Mozc ヘルパー (meltype_mozc_helper) と標準入出力で話す変換器 (Layer B)。</summary>
internal sealed class BenchMozc : IKanjiConverter, IDisposable
{
    private const char UnitSeparator = '\x1f';
    private const char RecordSeparator = '\x1e';
    private readonly Process _process;
    private readonly StreamWriter _input;
    private readonly StreamReader _output;
    private readonly object _gate = new();

    public BenchMozc(string helperPath, string? profileDirectory)
    {
        var start = new ProcessStartInfo(helperPath)
        {
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            StandardInputEncoding = new UTF8Encoding(false),
            StandardOutputEncoding = new UTF8Encoding(false),
            StandardErrorEncoding = new UTF8Encoding(false),
        };
        if (!string.IsNullOrEmpty(profileDirectory)) start.ArgumentList.Add(profileDirectory);
        start.Environment["LC_ALL"] = "C.UTF-8";
        _process = Process.Start(start) ?? throw new InvalidOperationException("mozc helper を起動できません");
        _input = _process.StandardInput;
        _input.AutoFlush = true;
        _output = _process.StandardOutput;
        _process.ErrorDataReceived += (_, _) => { };
        _process.BeginErrorReadLine();
        var ready = _output.ReadLine();
        if (ready != "READY") throw new InvalidOperationException("mozc helper が READY を返しません: " + ready);
    }

    public string? Convert(string hiragana) =>
        ConvertClauses(hiragana) is { Count: > 0 } clauses ? string.Concat(clauses.Select(c => c.Text)) : null;

    public IReadOnlyList<ConversionClause>? ConvertClauses(string hiragana, string? context = null)
    {
        var line = Send(context ?? "", hiragana);
        if (string.IsNullOrEmpty(line)) return null;
        var clauses = new List<ConversionClause>();
        foreach (var record in line.Split(RecordSeparator))
        {
            var fields = record.Split(UnitSeparator);
            if (fields.Length < 2 || fields[0].Length == 0) continue;
            clauses.Add(new ConversionClause(fields[0], fields[1]));
        }
        return clauses.Count == 0 ? null : clauses;
    }

    private string? Send(string context, string reading)
    {
        lock (_gate)
        {
            _input.Write("C\t" + context + "\t" + reading + "\n");
            return _output.ReadLine();
        }
    }

    public void Dispose()
    {
        try { _input.WriteLine("Q"); } catch { }
        try { if (!_process.WaitForExit(2000)) _process.Kill(); } catch { }
        _process.Dispose();
    }
}
