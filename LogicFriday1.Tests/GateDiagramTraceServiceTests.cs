using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class GateDiagramTraceServiceTests
{
    [Fact]
    public void Evaluate_ComputesAndGateOutputFromInputValues()
    {
        var gateDiagramFunction = CreateAndGateDiagramFunction();

        var result = GateDiagramTraceService.Evaluate(
            gateDiagramFunction,
            new Dictionary<string, int>
            {
                ["A"] = 1,
                ["B"] = 1
            });

        result.ShouldSatisfyAllConditions(
            static value => value.OutputValues["F"].ShouldBe(1),
            static value => value.ItemValues[3].ShouldBe(1),
            static value => value.ConnectionValues[
                new GateDiagramConnectionReference(4, GateDiagramConnectionKind.Input, 0)]
                .ShouldBe(1));
    }

    [Fact]
    public void Evaluate_UsesZeroForMissingInputValues()
    {
        var gateDiagramFunction = CreateAndGateDiagramFunction();

        var result = GateDiagramTraceService.Evaluate(
            gateDiagramFunction,
            new Dictionary<string, int>
            {
                ["A"] = 1
            });

        result.OutputValues["F"].ShouldBe(0);
    }

    private static GateDiagramFunction CreateAndGateDiagramFunction()
    {
        return new GateDiagramFunction(
            ["A", "B"],
            ["F"],
            [["0"], ["0"], ["0"], ["1"]],
            "",
            [
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1),
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 80, "B", Id: 2),
                new GateDiagramItem(GatePaletteKind.And, 2, 120, 40, "", Id: 3),
                new GateDiagramItem(GatePaletteKind.Output, 1, 240, 40, "F", Id: 4)
            ],
            [
                new GateDiagramWire(
                    new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 0)),
                new GateDiagramWire(
                    new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 1)),
                new GateDiagramWire(
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(4, GateDiagramConnectionKind.Input, 0))
            ]);
    }
}
