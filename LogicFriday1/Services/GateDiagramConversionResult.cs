namespace LogicFriday1.Services;

public sealed record GateDiagramConversionResult(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues);
