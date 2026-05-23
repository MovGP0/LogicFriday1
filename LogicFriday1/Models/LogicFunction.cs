namespace LogicFriday1.Models;

public abstract record LogicFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText);

public sealed record TruthTableLogicFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText);

public sealed record LogicEquationFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText);

public sealed record GateDiagramFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText,
    IReadOnlyList<GateDiagramItem> Items,
    IReadOnlyList<GateDiagramWire> Wires)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText);
