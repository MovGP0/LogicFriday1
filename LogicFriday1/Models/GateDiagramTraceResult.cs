namespace LogicFriday1.Models;

public sealed record GateDiagramTraceResult(
    IReadOnlyDictionary<int, int> ItemValues,
    IReadOnlyDictionary<GateDiagramConnectionReference, int> ConnectionValues,
    IReadOnlyDictionary<string, int> OutputValues);
