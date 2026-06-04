using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class GateDiagramPackageInfoServiceTests
{
    [Fact]
    public void BuildPackageInfo_GroupsGatesByIcPackageCapacity()
    {
        var gateDiagramFunction = new GateDiagramFunction(
            ["A"],
            ["F"],
            [["0"], ["1"]],
            "",
            [
                new GateDiagramItem(GatePaletteKind.Not, 1, 0, 0, "", Id: 1),
                new GateDiagramItem(GatePaletteKind.Not, 1, 0, 0, "", Id: 2),
                new GateDiagramItem(GatePaletteKind.Nand, 2, 0, 0, "", Id: 3),
                new GateDiagramItem(GatePaletteKind.Nand, 2, 0, 0, "", Id: 4),
                new GateDiagramItem(GatePaletteKind.Nand, 2, 0, 0, "", Id: 5),
                new GateDiagramItem(GatePaletteKind.Nand, 2, 0, 0, "", Id: 6),
                new GateDiagramItem(GatePaletteKind.Nand, 2, 0, 0, "", Id: 7),
                new GateDiagramItem(GatePaletteKind.Xor, 2, 0, 0, "", Id: 8),
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 9),
                new GateDiagramItem(GatePaletteKind.Output, 1, 0, 0, "F", Id: 10)
            ],
            []);

        var packageInfo = GateDiagramPackageInfoService.BuildPackageInfo(gateDiagramFunction);

        packageInfo.ShouldSatisfyAllConditions(
            static value => value.ShouldContain("IC\tQty"),
            static value => value.ShouldContain("Hex Inverter\t1"),
            static value => value.ShouldContain("Quad 2-Input NAND\t2"),
            static value => value.ShouldContain("Quad 2-Input EXOR\t1"),
            static value => value.ShouldContain("TOTAL PACKAGES\t4"));
    }
}
