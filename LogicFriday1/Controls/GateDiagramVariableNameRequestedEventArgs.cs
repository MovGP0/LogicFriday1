using LogicFriday1.Models;

namespace LogicFriday1.Controls;

public sealed class GateDiagramVariableNameRequestedEventArgs(
    GatePaletteItem item,
    double x,
    double y,
    Action<string> addItem) : EventArgs
{
    public GatePaletteItem Item { get; } = item;

    public double X { get; } = x;

    public double Y { get; } = y;

    public void AddItem(string variableName) => addItem(variableName);
}
