namespace LogicFriday1.Services;

public sealed record LogicEquationParseResult(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText);
