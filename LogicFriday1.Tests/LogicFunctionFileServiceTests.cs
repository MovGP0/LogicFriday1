using LogicFriday1.Models;
using LogicFriday1.Services;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class LogicFunctionFileServiceTests : IDisposable
{
    private readonly string _tempDirectory = Path.Combine(
        Directory.GetCurrentDirectory(),
        ".temp",
        Guid.NewGuid().ToString("N"));

    public LogicFunctionFileServiceTests()
    {
        Directory.CreateDirectory(_tempDirectory);
    }

    [Fact]
    public void SaveAndLoad_RoundTripsTruthTableFunction()
    {
        var service = new LogicFunctionFileService();
        var filePath = Path.Combine(_tempDirectory, "xor.lfcn");
        var function = new TruthTableLogicFunction(
            ["A", "B"],
            ["F"],
            [
                ["0"],
                ["1"],
                ["1"],
                ["0"]
            ],
            "F = A' B + A B';",
            new MinimizedLogicFunction(
                [new MinimizedProductTerm("01", ["1"])],
                "F = A' B;",
                ".i 2"));

        service.Save(filePath, function);
        var loadedFunction = service.Load(filePath);

        loadedFunction.ShouldSatisfyAllConditions(
            static loaded => loaded.ShouldBeOfType<TruthTableLogicFunction>(),
            static loaded => loaded.InputNames.ShouldBe(["A", "B"]),
            static loaded => loaded.OutputNames.ShouldBe(["F"]),
            static loaded => loaded.OutputValues.Select(static row => row[0]).ShouldBe(["0", "1", "1", "0"]),
            static loaded => loaded.EquationText.ShouldBe("F = A' B + A B';"),
            static loaded => loaded.MinimizedFunction!.Products[0].InputPattern.ShouldBe("01"));
    }

    [Fact]
    public void SaveAndLoad_RoundTripsGateDiagramFunction()
    {
        var service = new LogicFunctionFileService();
        var filePath = Path.Combine(_tempDirectory, "gate.lfcn");
        var function = new GateDiagramFunction(
            ["A"],
            ["F"],
            [
                ["0"],
                ["1"]
            ],
            "F = A;",
            [new GateDiagramItem(GatePaletteKind.Input, 0, 10, 20, "A", Id: 1)],
            [
                new GateDiagramWire(
                    new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Input, 0),
                    [new GateDiagramWirePoint(30, 40)])
            ],
            IsMappedGateDiagram: true);

        service.Save(filePath, function);
        var loadedFunction = service.Load(filePath);

        loadedFunction.ShouldSatisfyAllConditions(
            static loaded => loaded.ShouldBeOfType<GateDiagramFunction>(),
            static loaded => ((GateDiagramFunction)loaded).Items[0].Label.ShouldBe("A"),
            static loaded => ((GateDiagramFunction)loaded).Wires[0].RoutePoints[0].X.ShouldBe(30),
            static loaded => ((GateDiagramFunction)loaded).IsMappedGateDiagram.ShouldBeTrue(),
            static loaded => loaded.OutputValues.Select(static row => row[0]).ShouldBe(["0", "1"]));
    }

    [Fact]
    public void Load_WithInvalidHeader_ThrowsLogicFunctionFileException()
    {
        var service = new LogicFunctionFileService();
        var filePath = Path.Combine(_tempDirectory, "invalid.lfcn");
        File.WriteAllText(filePath, "not a logic function");

        var exception = Should.Throw<LogicFunctionFileException>(() => service.Load(filePath));

        exception.Message.ShouldBe("The file is not a LogicFriday1 port logic function file.");
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDirectory))
        {
            Directory.Delete(_tempDirectory, recursive: true);
        }
    }
}
