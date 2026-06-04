using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class TruthTableImporterTests
{
    [Fact]
    public void Import_AndGateCsv_LoadsTriValueRows()
    {
        AssertGateImport(
            AndGateCsv,
            [
                ["0"],
                ["X"],
                ["X"],
                ["1"]
            ]);
    }

    [Fact]
    public void Import_FullAdderCsv_LoadsExpectedOutputValues()
    {
        var result = TruthTableImporter.Import(FullAdderCsv);

        result.ShouldSatisfyAllConditions(
            static import => import.InputNames.ShouldBe(["CIn", "A", "B"]),
            static import => import.OutputNames.ShouldBe(["C", "S"]),
            static import => import.OutputValues.ShouldBe(FullAdderOutputValues()));
    }

    [Fact]
    public void Import_NandGateCsv_LoadsTriValueRows()
    {
        AssertGateImport(
            NandGateCsv,
            [
                ["1"],
                ["1"],
                ["1"],
                ["X"]
            ]);
    }

    [Fact]
    public void Import_NorGateCsv_LoadsTriValueRows()
    {
        AssertGateImport(
            NorGateCsv,
            [
                ["1"],
                ["X"],
                ["X"],
                ["0"]
            ]);
    }

    [Fact]
    public void Import_OrGateCsv_LoadsTriValueRows()
    {
        AssertGateImport(
            OrGateCsv,
            [
                ["X"],
                ["1"],
                ["1"],
                ["1"]
            ]);
    }

    private const string AndGateCsv = """
        A,B,,F
        0,X,,0
        X,0,,0
        1,X,,X
        X,1,,X
        1,1,,1
        """;

    private const string FullAdderCsv = """
        CIn,A,B,,C,S
        1,1,X,,1,0
        1,X,1,,1,0
        X,1,1,,1,0
        1,1,1,,0,1
        0,0,1,,0,1
        0,1,0,,0,1
        1,0,0,,0,1
        """;

    private const string NandGateCsv = """
        A,B,,F
        0,X,,1
        X,0,,1
        1,X,,X
        X,1,,X
        1,1,,0
        """;

    private const string NorGateCsv = """
        A,B,,F
        0,0,,1
        0,X,,X
        X,0,,X
        1,X,,0
        X,1,,0
        """;

    private const string OrGateCsv = """
        A,B,,F
        1,X,,1
        X,1,,1
        0,X,,X
        X,0,,X
        0,0,,0
        """;

    private static void AssertGateImport(string csv, string[][] expectedOutputValues)
    {
        var result = TruthTableImporter.Import(csv);

        result.ShouldSatisfyAllConditions(
            static import => import.InputNames.ShouldBe(["A", "B"]),
            static import => import.OutputNames.ShouldBe(["F"]),
            import => import.OutputValues.ShouldBe(expectedOutputValues));
    }

    private static string[][] FullAdderOutputValues()
    {
        return
        [
            ["0", "0"],
            ["0", "1"],
            ["0", "1"],
            ["1", "0"],
            ["0", "1"],
            ["1", "0"],
            ["1", "0"],
            ["1", "1"]
        ];
    }
}
