using System.Reflection;
using Avalonia.Media;
using LogicFriday1.Controls;
using LogicFriday1.Models;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class GateDiagramSurfaceTests
{
    [Fact]
    public void GetWireColorIndexes_WiresSharingConnection_UseSameColorIndex()
    {
        var sharedOutput = new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0);
        GateDiagramWire[] wires =
        [
            new(
                sharedOutput,
                new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Input, 0)),
            new(
                sharedOutput,
                new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 0)),
            new(
                new GateDiagramConnectionReference(4, GateDiagramConnectionKind.Output, 0),
                new GateDiagramConnectionReference(5, GateDiagramConnectionKind.Input, 0))
        ];

        var colorIndexes = InvokeGetWireColorIndexes(wires);

        colorIndexes.ShouldSatisfyAllConditions(
            static value => value[0].ShouldBe(value[1]),
            static value => value[2].ShouldNotBe(value[0]));
    }

    [Fact]
    public void GetWireColorIndexes_TransitivelyConnectedWires_UseSameColorIndex()
    {
        var sharedInput = new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Input, 0);
        GateDiagramWire[] wires =
        [
            new(
                new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                sharedInput),
            new(
                sharedInput,
                new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 0))
        ];

        var colorIndexes = InvokeGetWireColorIndexes(wires);

        colorIndexes[0].ShouldBe(colorIndexes[1]);
    }

    [Fact]
    public void GetWireBrush_FirstThirtyTwoIndexes_UseUniquePaletteColors()
    {
        var colors = Enumerable
            .Range(0, 32)
            .Select(InvokeGetWireBrushColor)
            .ToArray();

        colors.Distinct().Count().ShouldBe(32);
    }

    [Fact]
    public void GetWireBrush_IndexAfterPaletteSize_WrapsToFirstPaletteColor()
    {
        InvokeGetWireBrushColor(32).ShouldBe(InvokeGetWireBrushColor(0));
    }

    private static IReadOnlyList<int> InvokeGetWireColorIndexes(IList<GateDiagramWire> wires)
    {
        var method = typeof(GateDiagramSurface).GetMethod(
            "GetWireColorIndexes",
            BindingFlags.NonPublic | BindingFlags.Static);

        method.ShouldNotBeNull();
        return ((IReadOnlyList<int>?)method.Invoke(null, [wires])).ShouldNotBeNull();
    }

    private static Color InvokeGetWireBrushColor(int colorIndex)
    {
        var method = typeof(GateDiagramSurface).GetMethod(
            "GetWireBrush",
            BindingFlags.NonPublic | BindingFlags.Static);

        method.ShouldNotBeNull();
        var brush = ((SolidColorBrush?)method.Invoke(null, [colorIndex])).ShouldNotBeNull();
        return brush.Color;
    }
}
