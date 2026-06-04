using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class TruthTableImporterTests
{
    [Fact]
    public void Import_FullAdderCsv_LoadsExpectedOutputValues()
    {
        var result = TruthTableImporter.Import(FullAdderCsv);

        result.ShouldSatisfyAllConditions(
            static import => import.InputNames.ShouldBe(["CIn", "A", "B"]),
            static import => import.OutputNames.ShouldBe(["C", "S"]),
            static import => import.OutputValues.ShouldBe(FullAdderOutputValues()));
    }

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
