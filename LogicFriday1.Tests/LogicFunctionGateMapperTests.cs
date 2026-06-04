using LogicFriday1.Models;
using LogicFriday1.Services;
using LogicFriday1.Sis;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class LogicFunctionGateMapperTests
{
    [Fact]
    public void BuildGateDiagramFunction_FlatMappedLevels_UsesDependencyLayersAndGridAlignedOutputs()
    {
        var source = new TruthTableLogicFunction(
            ["A", "B", "C"],
            ["F"],
            Enumerable
                .Range(0, 8)
                .Select(static _ => new[] { "0" })
                .ToArray(),
            "");
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                new SisMappedGate
                {
                    Id = "g1",
                    Kind = "and",
                    Inputs = ["A", "B"],
                    Output = "n1",
                    Level = 1
                },
                new SisMappedGate
                {
                    Id = "g2",
                    Kind = "or",
                    Inputs = ["n1", "C"],
                    Output = "F",
                    Level = 1
                }
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var firstGate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.And);
        var secondGate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.Or);
        var output = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.Output);
        gateDiagram.ShouldSatisfyAllConditions(
            _ => firstGate.X.ShouldBeLessThan(secondGate.X),
            _ => secondGate.X.ShouldBeLessThan(output.X),
            _ => output.Y.ShouldBe(secondGate.Y),
            _ => output.X.ShouldBe(output.X / 20d * 20d),
            _ => output.Y.ShouldBe(output.Y / 20d * 20d));
    }
}
