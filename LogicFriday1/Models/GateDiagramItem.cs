namespace LogicFriday1.Models;

public sealed record GateDiagramItem(
    GatePaletteKind Kind,
    int InputCount,
    double X,
    double Y,
    string Label);
