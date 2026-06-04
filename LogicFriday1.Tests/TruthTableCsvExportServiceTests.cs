using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class TruthTableCsvExportServiceTests
{
    [Fact]
    public void Export_WritesFullTruthTableCsv()
    {
        var logicFunction = new TruthTableLogicFunction(
            ["A", "B"],
            ["F"],
            [
                ["0"],
                ["1"],
                ["X"],
                ["0"]
            ],
            "");

        var csv = TruthTableCsvExportService.Export(logicFunction);

        csv.ShouldBe("A,B,,F\r\n0,0,,0\r\n0,1,,1\r\n1,0,,X\r\n1,1,,0\r\n");
    }

    [Fact]
    public void Export_WhenMinimizedRequested_WritesMinimizedProductRows()
    {
        var logicFunction = new TruthTableLogicFunction(
            ["A", "B"],
            ["F"],
            [
                ["0"],
                ["1"],
                ["1"],
                ["0"]
            ],
            "",
            new MinimizedLogicFunction(
                [new MinimizedProductTerm("1-", ["1"])],
                "",
                ""));

        var csv = TruthTableCsvExportService.Export(logicFunction, useMinimizedFunction: true);

        csv.ShouldBe("A,B,,F\r\n1,X,,1\r\n");
    }

    [Fact]
    public void Export_WhenHeaderRequiresEscaping_WritesQuotedCsvCell()
    {
        var logicFunction = new TruthTableLogicFunction(
            ["A,0"],
            ["F\"1"],
            [
                ["1"],
                ["0"]
            ],
            "");

        var csv = TruthTableCsvExportService.Export(logicFunction);

        csv.ShouldBe("\"A,0\",,\"F\"\"1\"\r\n0,,1\r\n1,,0\r\n");
    }
}
