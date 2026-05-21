namespace LogicFriday1.Services;

public sealed record TruthTableImportResult(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues);
