namespace Espresso;

public readonly record struct VariableAnalysis(
    int[] PartZeros,
    int[] VarZeros,
    bool[] IsUnate,
    int VarsActive,
    int VarsUnate,
    int Best);
