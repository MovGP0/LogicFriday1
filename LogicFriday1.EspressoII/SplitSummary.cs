namespace Espresso;

public readonly record struct SplitSummary(
    int VarsActive,
    int VarsUnate,
    int Best,
    int BestVarZeros);
