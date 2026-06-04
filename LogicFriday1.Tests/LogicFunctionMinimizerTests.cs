using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class LogicFunctionMinimizerTests
{
    [Theory]
    [InlineData(true, true)]
    [InlineData(true, false)]
    [InlineData(false, true)]
    [InlineData(false, false)]
    public void Minimize_FullAdderTruthTable_ReturnsExpectedEquations(
        bool useExactMode,
        bool minimizeOutputsIndependently)
    {
        var minimizedFunction = LogicFunctionMinimizer.Minimize(
            CreateFullAdderFunction(),
            new MinimizeOptions(useExactMode, minimizeOutputsIndependently));

        CanonicalizeEquationText(minimizedFunction.EquationText).ShouldBe(CanonicalizeEquationText(ExpectedMinimizedEquationText));
    }

    private const string ExpectedMinimizedEquationText = """
        Minimized:
        C = CIn A + CIn B + A B;
        S = CIn A B + CIn' A' B + CIn' A B' + CIn A' B';
        """;

    private static TruthTableLogicFunction CreateFullAdderFunction()
    {
        return new TruthTableLogicFunction(
            ["CIn", "A", "B"],
            ["C", "S"],
            FullAdderOutputValues(),
            "");
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

    private static string CanonicalizeEquationText(string equationText)
    {
        return string.Join(
            Environment.NewLine,
            equationText
                .ReplaceLineEndings()
                .Split(Environment.NewLine, StringSplitOptions.RemoveEmptyEntries)
                .Select(CanonicalizeEquationLine));
    }

    private static string CanonicalizeEquationLine(string line)
    {
        var equationParts = line.Split('=', 2);
        if (equationParts.Length != 2)
        {
            return line.Trim();
        }

        var expressionParts = equationParts[1].Trim().TrimEnd(';')
            .Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries)
            .Order(StringComparer.Ordinal)
            .ToArray();

        return $"{equationParts[0].Trim()} = {string.Join(" + ", expressionParts)};";
    }
}
