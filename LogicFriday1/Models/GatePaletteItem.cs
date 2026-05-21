namespace LogicFriday1.Models;

public sealed record GatePaletteItem(
    string Label,
    GatePaletteKind Kind,
    int CommandId,
    int InputCount,
    string SourceReference);
