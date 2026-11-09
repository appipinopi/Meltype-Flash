// SPDX-License-Identifier: GPL-3.0-or-later
using System.Diagnostics;
using System.Text.Json;
using System.Text.Json.Serialization;
using Meltype.Composition;
using Meltype.Config;
using Meltype.Detection;

var cases = JsonSerializer.Deserialize(File.ReadAllText(args[0]), ReportJson.Default.CaseArray)!;
var samples = int.Parse(args[1]);
var warmups = int.Parse(args[2]);
var progress = Environment.GetEnvironmentVariable("MELTYPE_BENCH_PROGRESS") == "1";
foreach (var c in cases)
{
    for (var sample = 0; sample < warmups + samples; sample++)
    {
        // Each run has fresh in-memory learning/history; bundled dictionaries are cached.
        var detector = CompositionDetector.CreateDefault(args.Length > 3 ? Path.Combine(args[3], "dictionaries") : null);
        // Rust loads this same bundled SCOWL word list. Disable OS APIs, not
        // the built-in spelling dictionary used by the original product.
        detector.SpellChecker = BuiltInWordChecker.Shared;
        var languages = new LanguageMemory(null);
        detector.Memory = languages;
        var settings = new Settings { LiveConversion = false, PredictiveCandidates = false, LearningEnabled = false }.Normalize();
        // Upstream 1.5.0 always uses the scored reading lattice; the former
        // ScoredSegmentation switch no longer exists.
        var options = new CompositionOptions
        {
            Languages = languages, Candidates = CandidateDictionary.Load(null),
            ContextRules = ContextRules.Load(null), History = new ConversionHistory(null),
            Misspellings = MisspellingDictionary.Load(null), RomajiTypos = RomajiTypoCorrector.Load(detector.Romaji),
            LiveConversion = () => false, Learning = () => false,
        };
        var session = new MeltypeSession(detector, new NullConverter(), options, () => settings);
        session.CanDeleteSurrounding = true;
        var responses = new List<string>(c.keys.Length);
        var keysMs = new List<double>(c.keys.Length);
        var started = Stopwatch.GetTimestamp();
        foreach (var key in c.keys)
        {
            var t = Stopwatch.GetTimestamp();
            var result = session.HandleKey(key[0], key[1] == 0 ? null : (char)key[1],
                (key[2] & 1) != 0, (key[2] & 2) != 0, (key[2] & 4) != 0, false);
            var response = result.ToJson();
            Bench.Sink = response;
            keysMs.Add(Stopwatch.GetElapsedTime(t).TotalMilliseconds);
            responses.Add(response);
        }
        var totalMs = Stopwatch.GetElapsedTime(started).TotalMilliseconds;
        if (sample >= warmups)
            Console.WriteLine(JsonSerializer.Serialize(new Row(c.name, sample-warmups, totalMs, keysMs, responses), ReportJson.Default.Row));
    }
    if (progress) Console.Error.WriteLine($"Completed {c.name}");
}
sealed record Case(string name, int[][] keys, bool scored = false);
sealed record Row([property: JsonPropertyName("case")] string name, int sample, double total_ms, List<double> keys_ms, List<string> responses);
static class Bench { public static string? Sink; }
sealed class NullConverter : IKanjiConverter
{
    public string? Convert(string reading) => null;
    public IReadOnlyList<ConversionClause>? ConvertClauses(string reading, string? context = null) => null;
}
[JsonSerializable(typeof(Case[]))]
[JsonSerializable(typeof(Row))]
partial class ReportJson : JsonSerializerContext;
