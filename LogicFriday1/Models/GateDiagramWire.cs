namespace LogicFriday1.Models;

public enum GateDiagramConnectionKind
{
    Input,
    Output
}

public sealed record GateDiagramConnectionReference(
    int ItemId,
    GateDiagramConnectionKind Kind,
    int PinIndex);

public sealed record GateDiagramConnectionPoint(
    GateDiagramConnectionReference Reference,
    double X,
    double Y);

public sealed record GateDiagramWirePoint(
    double X,
    double Y);

public sealed record GateDiagramWire(
    GateDiagramConnectionReference Start,
    GateDiagramConnectionReference End,
    IReadOnlyList<GateDiagramWirePoint> RoutePoints)
{
    public GateDiagramWire(
        GateDiagramConnectionReference start,
        GateDiagramConnectionReference end)
        : this(start, end, [])
    {
    }
}
