using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class GateDiagramSvgExportServiceTests
{
    [Fact]
    public void CanExport_ReturnsTrueOnlyForGateDiagramWithItems()
    {
        var gateDiagram = CreateGateDiagramFunction();
        var emptyGateDiagram = gateDiagram with
        {
            Items = []
        };

        GateDiagramSvgExportService.CanExport(gateDiagram).ShouldSatisfyAllConditions(
            static canExport => canExport.ShouldBeTrue(),
            _ => GateDiagramSvgExportService.CanExport(emptyGateDiagram).ShouldBeFalse(),
            _ => GateDiagramSvgExportService.CanExport(null).ShouldBeFalse());
    }

    [Fact]
    public void Export_WritesSvgWithItemsAndWires()
    {
        var gateDiagram = CreateGateDiagramFunction();

        var svg = GateDiagramSvgExportService.Export(gateDiagram);

        svg.ShouldSatisfyAllConditions(
            static value => value.ShouldStartWith("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            static value => value.ShouldContain("<polyline points=\"40,25 120,25\"/>"),
            static value => value.ShouldContain(">A</text>"),
            static value => value.ShouldContain(">OUTPUT</text>"));
    }

    private static GateDiagramFunction CreateGateDiagramFunction()
    {
        return new GateDiagramFunction(
            ["A"],
            ["F"],
            [
                ["0"],
                ["1"]
            ],
            "F = A;",
            [
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1),
                new GateDiagramItem(GatePaletteKind.Output, 1, 120, 0, "F", Id: 2)
            ],
            [
                new GateDiagramWire(
                    new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Input, 0))
            ]);
    }
}
