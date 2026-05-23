namespace LogicFriday1.Models;

public abstract record LogicFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText,
    MinimizedLogicFunction? MinimizedFunction = null);

public sealed record TruthTableLogicFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText,
    MinimizedLogicFunction? MinimizedFunction = null)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText, MinimizedFunction);

public sealed record LogicEquationFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText,
    MinimizedLogicFunction? MinimizedFunction = null)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText, MinimizedFunction);

public sealed record GateDiagramFunction(
    string[] InputNames,
    string[] OutputNames,
    IReadOnlyList<string[]> OutputValues,
    string EquationText,
    IReadOnlyList<GateDiagramItem> Items,
    IReadOnlyList<GateDiagramWire> Wires,
    MinimizedLogicFunction? MinimizedFunction = null)
    : LogicFunction(InputNames, OutputNames, OutputValues, EquationText, MinimizedFunction);

public sealed record MinimizedLogicFunction(
    IReadOnlyList<MinimizedProductTerm> Products,
    string EquationText,
    string PlaText);

public sealed record MinimizedProductTerm(
    string InputPattern,
    string[] OutputValues);
