// SPDX-License-Identifier: GPL-3.0-or-later
// 1万文コアベンチ用ハーネス 世代2 (v1.6 系: CompositionServices + MeltypeRuntime.CreateSession)
//
//   使い方: CoreBench <cases.jsonl> <out.jsonl> [mozcHelperPath|-] [profileDir]
//   cases.jsonl: 1行1件 {"id":1,"typed":"kyouhaii..."} (ローマ字で打鍵する文字列)
//   out.jsonl:   1行1件 {"id":1,"enter":"…","space":"…","ms":1.23}
//   mozcHelperPath が "-" なら変換器なし (Layer A)。
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Meltype.Composition;

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
        var dataDirectory = Path.Combine(Path.GetTempPath(), "meltype-core-bench-" + Environment.ProcessId);
        Directory.CreateDirectory(dataDirectory);
        BenchMozc? mozc = helper == "-" ? null : new BenchMozc(helper, profile);
        MeltypeRuntime? runtime = null;
        try
        {
            IKanjiConverter converter = mozc is null ? new NullConverter() : mozc;
            runtime = new MeltypeRuntime(dataDirectory);
            using var output = new StreamWriter(args[1], false, new UTF8Encoding(false));
            var index = 0;
            foreach (var line in File.ReadLines(args[0]))
            {
                if (string.IsNullOrWhiteSpace(line)) continue;
                using var document = JsonDocument.Parse(line);
                var root = document.RootElement;
                var id = root.GetProperty("id").GetInt32();
                var typed = root.GetProperty("typed").GetString() ?? "";
                // セッションの作成は計測に含めない (打鍵から Space・Enter までを測る)。
                var session = runtime.CreateSession(converter);
                var watch = Stopwatch.StartNew();
                var (enter, space) = Run(session, typed);
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
            runtime?.Dispose();
            mozc?.Dispose();
        }
        return 0;
    }

    private static (string Enter, string Space) Run(MeltypeSession session, string typed)
    {
        var committed = new StringBuilder();
        foreach (var c in typed)
        {
            var result = session.HandleKey(VirtualKey(c), c, false, false, false, false);
            Apply(committed, result.Commits);
        }
        var spaceResult = session.HandleKey(0x20, ' ', false, false, false, false);
        var spaceBuffer = new StringBuilder(committed.ToString());
        Apply(spaceBuffer, spaceResult.Commits);
        var space = spaceResult.View?.Text ?? spaceBuffer.ToString();
        var enterResult = session.HandleKey(0x0D, null, false, false, false, false);
        var enterBuffer = new StringBuilder(spaceBuffer.ToString());
        Apply(enterBuffer, enterResult.Commits);
        return (enterBuffer.ToString(), space);
    }

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
        var line = Request(context ?? "", hiragana);
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

    private string? Request(string context, string reading)
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
