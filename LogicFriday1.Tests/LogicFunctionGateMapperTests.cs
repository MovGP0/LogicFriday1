using System.Reflection;
using LogicFriday1.Models;
using LogicFriday1.Services;
using LogicFriday1.Sis;
using LogicFriday1.ViewModels;
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
            _ => output.X.ShouldBe(secondGate.X + 160),
            _ => output.Y.ShouldBe(secondGate.Y),
            _ => output.X.ShouldBe(output.X / 20d * 20d),
            _ => output.Y.ShouldBe(output.Y / 20d * 20d));
    }

    [Fact]
    public void BuildGateDiagramFunction_NonTopologicalMappedNetwork_UsesDependencyLayersAndWires()
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
                    Id = "g2",
                    Kind = "or",
                    Inputs = ["n1", "C"],
                    Output = "n2",
                    Level = 1
                },
                new SisMappedGate
                {
                    Id = "out",
                    Kind = "buf",
                    Inputs = ["n2"],
                    Output = "F",
                    Level = 1
                },
                new SisMappedGate
                {
                    Id = "g1",
                    Kind = "and",
                    Inputs = ["A", "B"],
                    Output = "n1",
                    Level = 1
                }
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var firstGate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.And);
        var secondGate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.Or);
        var output = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.Output);
        gateDiagram.ShouldSatisfyAllConditions(
            _ => firstGate.X.ShouldBe(180),
            _ => secondGate.X.ShouldBe(340),
            _ => output.X.ShouldBe(500),
            _ => gateDiagram.Wires.ShouldContain(wire => wire.Start.ItemId == firstGate.Id
                && wire.End.ItemId == secondGate.Id),
            _ => gateDiagram.Wires.ShouldContain(wire => wire.Start.ItemId == secondGate.Id
                && wire.End.ItemId == output.Id));
    }

    [Fact]
    public void BuildGateDiagramFunction_SameLayerGates_OrdersRowsByInputBarycenter()
    {
        var source = CreateEmptyTruthTableFunction(["A", "B"], ["F", "G"]);
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                Gate("gB", "and", ["B", "B"], "G"),
                Gate("gA", "and", ["A", "A"], "F")
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var gateA = gateDiagram.Items.Single(item => item.Kind == GatePaletteKind.And && DrivesOutput(gateDiagram, item, "F"));
        var gateB = gateDiagram.Items.Single(item => item.Kind == GatePaletteKind.And && DrivesOutput(gateDiagram, item, "G"));
        gateA.Y.ShouldBeLessThan(gateB.Y);
    }

    [Fact]
    public void BuildGateDiagramFunction_CommutativeGate_ReordersInputPinsBySourceOrder()
    {
        var source = CreateEmptyTruthTableFunction(["A", "B"], ["F"]);
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                Gate("g1", "and", ["B", "A"], "F")
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var gate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.And);
        var inputA = gateDiagram.Items.Single(static item => item.Label == "A");
        var inputB = gateDiagram.Items.Single(static item => item.Label == "B");
        gateDiagram.ShouldSatisfyAllConditions(
            _ => WireTo(gateDiagram, inputA, gate).End.PinIndex.ShouldBe(0),
            _ => WireTo(gateDiagram, inputB, gate).End.PinIndex.ShouldBe(1));
    }

    [Fact]
    public void BuildGateDiagramFunction_MuxGate_DoesNotReorderInputPins()
    {
        var source = CreateEmptyTruthTableFunction(["A", "B", "C"], ["F"]);
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                Gate("g1", "mux", ["B", "A", "C"], "F")
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var gate = gateDiagram.Items.Single(static item => item.Kind == GatePaletteKind.Mux);
        var inputA = gateDiagram.Items.Single(static item => item.Label == "A");
        var inputB = gateDiagram.Items.Single(static item => item.Label == "B");
        var inputC = gateDiagram.Items.Single(static item => item.Label == "C");
        gateDiagram.ShouldSatisfyAllConditions(
            _ => WireTo(gateDiagram, inputB, gate).End.PinIndex.ShouldBe(0),
            _ => WireTo(gateDiagram, inputA, gate).End.PinIndex.ShouldBe(1),
            _ => WireTo(gateDiagram, inputC, gate).End.PinIndex.ShouldBe(2));
    }

    [Fact]
    public void BuildGateDiagramFunction_GeneratedWires_UseRoutedVirtualPoints()
    {
        var source = CreateEmptyTruthTableFunction(["A", "B", "C"], ["F"]);
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                Gate("g1", "and", ["A", "B"], "n1"),
                Gate("g2", "or", ["n1", "C"], "F")
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        gateDiagram.Wires.ShouldAllBe(static wire => wire.RoutePoints.Count >= 2);
    }

    [Fact]
    public void BuildGateDiagramFunction_SharedBoundaryWires_UseSeparatedGridLanes()
    {
        var source = CreateEmptyTruthTableFunction(["A", "B", "C", "D", "E", "G"], ["F"]);
        var mapped = new SisMappedNetwork
        {
            Gates =
            [
                Gate("gA", "and", ["A", "A"], "nA"),
                Gate("gB", "and", ["B", "B"], "nB"),
                Gate("gC", "and", ["C", "C"], "nC"),
                Gate("gD", "and", ["D", "D"], "nD"),
                Gate("gE", "and", ["E", "E"], "nE"),
                Gate("gG", "and", ["G", "G"], "nG"),
                Gate("gOut", "and", ["nA", "nB", "nC", "nD", "nE", "nG"], "F")
            ]
        };

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var outputDriver = gateDiagram.Items.Single(item => item.Kind == GatePaletteKind.And && DrivesOutput(gateDiagram, item, "F"));
        var incomingChannelXs = gateDiagram.Wires
            .Where(wire => wire.End.ItemId == outputDriver.Id)
            .Select(static wire => wire.RoutePoints[0].X)
            .Order()
            .ToArray();
        incomingChannelXs.ShouldSatisfyAllConditions(
            static value => value.Length.ShouldBe(6),
            static value => value.Distinct().Count().ShouldBe(6));
        foreach (var distance in incomingChannelXs.Zip(incomingChannelXs.Skip(1), static (left, right) => right - left))
        {
            distance.ShouldBeGreaterThanOrEqualTo(20);
        }
    }

    [Fact]
    public void BuildGateDiagramFunction_FullAdderNandNorMappedNetwork_UsesExpectedDependencyColumns()
    {
        var source = CreateMinimizedFullAdderFunction();
        var mapped = CreateFullAdderNandNorMappedNetwork();

        var gateDiagram = LogicFunctionGateMapper.BuildGateDiagramFunction(source, mapped);

        var columns = gateDiagram.Items
            .GroupBy(static item => item.X)
            .OrderBy(static column => column.Key)
            .Select(static column => column.ToArray())
            .ToArray();
        columns.ShouldSatisfyAllConditions(
            static value => value.Length.ShouldBe(9),
            static value => Count(value[0], GatePaletteKind.Input).ShouldBe(3),
            static value => Count(value[1], GatePaletteKind.Not).ShouldBe(3),
            static value => Count(value[1], GatePaletteKind.Nand).ShouldBe(1),
            static value => Count(value[1], GatePaletteKind.Nor).ShouldBe(1),
            static value => Count(value[2], GatePaletteKind.Nor).ShouldBe(1),
            static value => Count(value[2], GatePaletteKind.Nand).ShouldBe(2),
            static value => Count(value[3], GatePaletteKind.Nand).ShouldBe(2),
            static value => Count(value[4], GatePaletteKind.Nand).ShouldBe(1),
            static value => Count(value[5], GatePaletteKind.Not).ShouldBe(1),
            static value => Count(value[6], GatePaletteKind.Nand).ShouldBe(1),
            static value => Count(value[7], GatePaletteKind.Nand).ShouldBe(1),
            static value => Count(value[8], GatePaletteKind.Output).ShouldBe(2),
            static value => value[0].Length.ShouldBe(3),
            static value => value[1].Length.ShouldBe(5),
            static value => value[2].Length.ShouldBe(3),
            static value => value[3].Length.ShouldBe(2),
            static value => value[4].Length.ShouldBe(1),
            static value => value[5].Length.ShouldBe(1),
            static value => value[6].Length.ShouldBe(1),
            static value => value[7].Length.ShouldBe(1),
            static value => value[8].Length.ShouldBe(2));
    }

    [Fact]
    public void Map_FullAdderWithInverterNandNorDieArea_UsesOnlySelectedGateKinds()
    {
        var source = CreateMinimizedFullAdderFunction();
        var options = new MapToGatesDialogViewModel
        {
            UseInverter = true,
            UseNand2 = true,
            UseNand3 = false,
            UseNand4 = false,
            UseNor2 = true,
            UseNor3 = false,
            UseNor4 = false,
            UseXor2 = false,
            UseMux2 = false,
            UseAnd2 = false,
            UseAnd3 = false,
            UseAnd4 = false,
            UseOr2 = false,
            UseOr3 = false,
            UseOr4 = false,
            UseStandardLogicIcs = false,
            UseDieArea = true
        };

        var gateDiagram = LogicFunctionGateMapper.Map(source, options);

        var gateKinds = gateDiagram.Items
            .Where(static item => item.Kind is not GatePaletteKind.Input and not GatePaletteKind.Output)
            .Select(static item => item.Kind)
            .ToArray();
        gateKinds.ShouldSatisfyAllConditions(
            static value => value.ShouldNotContain(GatePaletteKind.And),
            static value => value.ShouldNotContain(GatePaletteKind.Or),
            static value => value.ShouldNotContain(GatePaletteKind.Xor),
            static value => value.ShouldNotContain(GatePaletteKind.Mux),
            static value => value.ShouldContain(GatePaletteKind.Not),
            static value => value.ShouldContain(GatePaletteKind.Nand),
            static value => value.ShouldContain(GatePaletteKind.Nor));
    }

    [Fact]
    public void BuildBlif_MinimizedFullAdder_UsesMinimizedProductRows()
    {
        var source = CreateMinimizedFullAdderFunction();

        var blif = InvokeBuildBlif(source);

        blif.ShouldSatisfyAllConditions(
            static value => value.ShouldContain(".names CIn A B C"),
            static value => value.ShouldContain("11- 1"),
            static value => value.ShouldContain("1-1 1"),
            static value => value.ShouldContain("-11 1"),
            static value => value.ShouldNotContain("011 1"),
            static value => value.ShouldNotContain("101 1"),
            static value => value.ShouldNotContain("110 1"));
    }

    [Fact]
    public void Map_FullAdderWithDefaultSettings_UsesOptimizedGateCount()
    {
        var source = CreateMinimizedFullAdderFunction();
        var options = new MapToGatesDialogViewModel();

        var gateDiagram = LogicFunctionGateMapper.Map(source, options);

        gateDiagram.Items
            .Count(static item => item.Kind is not GatePaletteKind.Input and not GatePaletteKind.Output)
            .ShouldBe(14);
    }

    private static int Count(
        IEnumerable<GateDiagramItem> items,
        GatePaletteKind kind)
    {
        return items.Count(item => item.Kind == kind);
    }

    private static bool DrivesOutput(
        GateDiagramFunction gateDiagram,
        GateDiagramItem source,
        string outputName)
    {
        var output = gateDiagram.Items.Single(item => item.Kind == GatePaletteKind.Output && item.Label == outputName);
        return gateDiagram.Wires.Any(wire => wire.Start.ItemId == source.Id && wire.End.ItemId == output.Id);
    }

    private static GateDiagramWire WireTo(
        GateDiagramFunction gateDiagram,
        GateDiagramItem source,
        GateDiagramItem target)
    {
        return gateDiagram.Wires.Single(wire => wire.Start.ItemId == source.Id && wire.End.ItemId == target.Id);
    }

    private static TruthTableLogicFunction CreateEmptyTruthTableFunction(
        IReadOnlyList<string> inputNames,
        IReadOnlyList<string> outputNames)
    {
        return new TruthTableLogicFunction(
            inputNames.ToArray(),
            outputNames.ToArray(),
            Enumerable
                .Range(0, 1 << inputNames.Count)
                .Select(_ => outputNames.Select(static _ => "0").ToArray())
                .ToArray(),
            "");
    }

    private static SisMappedNetwork CreateFullAdderNandNorMappedNetwork()
    {
        return new SisMappedNetwork
        {
            Gates =
            [
                Gate("g12", "not", ["n8"], "n9"),
                Gate("g7", "nand2", ["n1", "B"], "n4"),
                Gate("g1", "inv", ["CIn"], "nCIn"),
                Gate("g14", "nand2", ["C_drv", "n7"], "S_drv"),
                Gate("g4", "nand2", ["CIn", "A"], "n1"),
                Gate("g10", "nand2", ["n4", "n5"], "n7"),
                Gate("g2", "inv", ["A"], "nA"),
                Gate("outC", "buf", ["C_drv"], "C"),
                Gate("g13", "nand2", ["n9", "n1"], "C_drv"),
                Gate("g6", "nor2", ["nCIn", "nA"], "n3"),
                Gate("g3", "inv", ["B"], "nB"),
                Gate("g11", "nand2", ["n6", "n7"], "n8"),
                Gate("g8", "nand2", ["n2", "CIn"], "n5"),
                Gate("g5", "nor2", ["A", "B"], "n2"),
                Gate("outS", "buf", ["S_drv"], "S"),
                Gate("g9", "nand2", ["n3", "nB"], "n6")
            ]
        };
    }

    private static SisMappedGate Gate(
        string id,
        string kind,
        IReadOnlyList<string> inputs,
        string output)
    {
        return new SisMappedGate
        {
            Id = id,
            Kind = kind,
            Inputs = inputs,
            Output = output,
            Level = 1
        };
    }

    private static string InvokeBuildBlif(LogicFunction logicFunction)
    {
        var method = typeof(LogicFunctionGateMapper).GetMethod(
            "BuildBlif",
            BindingFlags.NonPublic | BindingFlags.Static);
        method.ShouldNotBeNull();
        return method.Invoke(null, [logicFunction]).ShouldBeOfType<string>();
    }

    private static TruthTableLogicFunction CreateMinimizedFullAdderFunction()
    {
        return new TruthTableLogicFunction(
            ["CIn", "A", "B"],
            ["C", "S"],
            [
                ["0", "0"],
                ["0", "1"],
                ["0", "1"],
                ["1", "0"],
                ["0", "1"],
                ["1", "0"],
                ["1", "0"],
                ["1", "1"]
            ],
            """
            Entered by truthtable:
            C = CIn' A B + CIn A' B + CIn A B' + CIn A B;
            S = CIn' A' B + CIn' A B' + CIn A' B' + CIn A B;
            """,
            new MinimizedLogicFunction(
                [
                    new MinimizedProductTerm("11-", ["1", "0"]),
                    new MinimizedProductTerm("1-1", ["1", "0"]),
                    new MinimizedProductTerm("-11", ["1", "0"]),
                    new MinimizedProductTerm("100", ["0", "1"]),
                    new MinimizedProductTerm("010", ["0", "1"]),
                    new MinimizedProductTerm("001", ["0", "1"]),
                    new MinimizedProductTerm("111", ["0", "1"])
                ],
                """
                Minimized:
                C = CIn A  + CIn B + A B;
                S = CIn A' B' + CIn' A B' + CIn' A' B + CIn A B;
                """,
                ""));
    }
}
