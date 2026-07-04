using System.Text;
using LogicFriday1.Models;
using LogicFriday1.Sis;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Services;

public static class LogicFunctionGateMapper
{
    private const double InputX = 20;

    private const double GateX = 180;

    private const double LevelSpacing = 160;

    private const double RowSpacing = 80;

    private const double RoutingGridStep = 20;

    private const double InputOutputOffsetX = 40;

    private const double ConstantOutputOffsetX = 55;

    private const double GateOutputOffsetX = 100;

    private const double PinCenterOffsetY = 25;

    public static GateDiagramFunction Map(
        LogicFunction logicFunction,
        MapToGatesDialogViewModel options)
    {
        ArgumentNullException.ThrowIfNull(logicFunction);
        ArgumentNullException.ThrowIfNull(options);

        var mapped = SisPort.MapBlifToGates(
            BuildBlif(logicFunction),
            options.BuildGenlib(),
            options.ToSisMapOptions());

        return BuildGateDiagramFunction(logicFunction, mapped);
    }

    private static string BuildBlif(LogicFunction logicFunction)
    {
        var builder = new StringBuilder()
            .AppendLine(".model logicfriday")
            .Append(".inputs");
        foreach (var input in logicFunction.InputNames)
        {
            builder.Append(' ').Append(input);
        }

        builder.AppendLine()
            .Append(".outputs");
        foreach (var output in logicFunction.OutputNames)
        {
            builder.Append(' ').Append(output);
        }

        builder.AppendLine();
        for (var outputIndex = 0; outputIndex < logicFunction.OutputNames.Length; outputIndex++)
        {
            builder.Append(".names");
            foreach (var input in logicFunction.InputNames)
            {
                builder.Append(' ').Append(input);
            }

            builder.Append(' ').AppendLine(logicFunction.OutputNames[outputIndex]);
            if (logicFunction.MinimizedFunction is { } minimizedFunction)
            {
                foreach (var product in minimizedFunction.Products.Where(product => product.OutputValues[outputIndex] == "1"))
                {
                    builder
                        .Append(product.InputPattern)
                        .AppendLine(" 1");
                }

                continue;
            }

            for (var term = 0; term < logicFunction.OutputValues.Count; term++)
            {
                if (logicFunction.OutputValues[term][outputIndex] != "1")
                {
                    continue;
                }

                builder
                    .Append(FormatInputPattern(term, logicFunction.InputNames.Length))
                    .AppendLine(" 1");
            }
        }

        builder.AppendLine(".end");
        return builder.ToString();
    }

    public static GateDiagramFunction BuildGateDiagramFunction(
        LogicFunction source,
        SisMappedNetwork mapped)
    {
        var layout = GateDiagramSugiyamaLayout.Build(source, mapped);

        return new GateDiagramFunction(
            [.. source.InputNames],
            [.. source.OutputNames],
            source.OutputValues.Select(static row => row.ToArray()).ToArray(),
            BuildMappedEquationText(source),
            layout.Items,
            layout.Wires,
            source.MinimizedFunction,
            IsMappedGateDiagram: true);
    }

    private static class GateDiagramSugiyamaLayout
    {
        private const int SweepCount = 8;

        public static LayoutResult Build(
            LogicFunction source,
            SisMappedNetwork mapped)
        {
            var bufferAliases = BuildBufferAliases(mapped);
            var signalLevels = BuildSignalLevels(source, mapped);
            var graph = new LayoutGraph(Math.Max(1, signalLevels.Values.DefaultIfEmpty(1).Max()));

            for (var index = 0; index < source.InputNames.Length; index++)
            {
                graph.AddInput(source.InputNames[index], index);
            }

            foreach (var gate in EnumerateDrawableGates(mapped))
            {
                graph.AddGate(
                    gate.Gate.Output,
                    ToGatePaletteKind(gate.Gate.Kind),
                    gate.Gate.Inputs.Count,
                    gate.Index,
                    signalLevels.GetValueOrDefault(gate.Gate.Output, Math.Max(1, gate.Gate.Level)));
            }

            foreach (var gate in EnumerateDrawableGates(mapped))
            {
                graph.ConnectGateInputs(gate.Gate.Output, gate.Gate.Inputs, bufferAliases);
            }

            for (var index = 0; index < source.OutputNames.Length; index++)
            {
                graph.AddOutput(source.OutputNames[index], ResolveBufferAlias(source.OutputNames[index], bufferAliases), index);
            }

            graph.Arrange();
            return graph.ToDiagram();
        }

        private static IEnumerable<MappedGateEntry> EnumerateDrawableGates(SisMappedNetwork mapped)
        {
            return mapped.Gates
                .Select(static (gate, index) => new MappedGateEntry(gate, index))
                .Where(static entry => !entry.Gate.Kind.Equals("buf", StringComparison.OrdinalIgnoreCase));
        }

        private sealed class LayoutGraph(int maxSignalLevel)
        {
            private readonly List<LayoutNode> nodes = [];

            private readonly List<LayoutEdge> edges = [];

            private readonly Dictionary<string, LayoutNode> signalDrivers = new(StringComparer.Ordinal);

            private double gateX = GateX;

            private double levelSpacing = LevelSpacing;

            public void AddInput(
                string signal,
                int sourceIndex)
            {
                AddSignalDriver(
                    signal,
                    new LayoutNode(
                        $"input:{sourceIndex}",
                        GatePaletteKind.Input,
                        0,
                        signal,
                        sourceIndex,
                        0,
                        NodeRole.Input));
            }

            public void AddGate(
                string outputSignal,
                GatePaletteKind kind,
                int mappedInputCount,
                int originalIndex,
                int layer)
            {
                AddSignalDriver(
                    outputSignal,
                    new LayoutNode(
                        $"gate:{originalIndex}:{outputSignal}",
                        kind,
                        GetInputCount(kind, mappedInputCount),
                        string.Empty,
                        originalIndex,
                        layer,
                        NodeRole.Gate));
            }

            public void AddOutput(
                string outputName,
                string driverSignal,
                int outputIndex)
            {
                var target = AddNode(new LayoutNode(
                    $"output:{outputIndex}",
                    GatePaletteKind.Output,
                    1,
                    outputName,
                    outputIndex,
                    maxSignalLevel + 1,
                    NodeRole.Output));
                if (signalDrivers.TryGetValue(driverSignal, out var source))
                {
                    AddEdge(source, target, 0, driverSignal);
                }
            }

            public void ConnectGateInputs(
                string outputSignal,
                IReadOnlyList<string> inputSignals,
                IReadOnlyDictionary<string, string> bufferAliases)
            {
                var target = signalDrivers[outputSignal];
                for (var inputIndex = 0; inputIndex < inputSignals.Count; inputIndex++)
                {
                    var signal = ResolveBufferAlias(inputSignals[inputIndex], bufferAliases);
                    if (signalDrivers.TryGetValue(signal, out var source))
                    {
                        AddEdge(source, target, inputIndex, signal);
                    }
                }
            }

            public void Arrange()
            {
                AssignInitialSlots();
                ReduceCrossings();
                AssignHorizontalSpacing();
                AssignCoordinates();
            }

            public LayoutResult ToDiagram()
            {
                var items = new List<GateDiagramItem>();
                var nodeItemIds = new Dictionary<LayoutNode, int>();
                var nextItemId = 1;
                var componentNumber = 1;
                foreach (var node in OrderedNodes())
                {
                    var componentLabel = node.NeedsComponentLabel
                        ? $"[{componentNumber++}]"
                        : string.Empty;
                    var item = new GateDiagramItem(
                        node.Kind,
                        node.InputCount,
                        node.X,
                        node.Y,
                        node.Label,
                        componentLabel,
                        nextItemId++);
                    items.Add(item);
                    nodeItemIds[node] = item.Id;
                }

                var routedEdges = RouteEdges();
                var wires = edges
                    .Where(edge => nodeItemIds.ContainsKey(edge.Source) && nodeItemIds.ContainsKey(edge.Target))
                    .Select(edge => new GateDiagramWire(
                        new GateDiagramConnectionReference(
                            nodeItemIds[edge.Source],
                            GateDiagramConnectionKind.Output,
                            0),
                        new GateDiagramConnectionReference(
                            nodeItemIds[edge.Target],
                            GateDiagramConnectionKind.Input,
                            edge.TargetPinIndex),
                        routedEdges[edge]))
                    .ToArray();

                return new LayoutResult(items, wires);
            }

            private void AddSignalDriver(
                string signal,
                LayoutNode node)
            {
                signalDrivers[signal] = AddNode(node);
            }

            private LayoutNode AddNode(LayoutNode node)
            {
                nodes.Add(node);
                return node;
            }

            private void AddEdge(
                LayoutNode source,
                LayoutNode target,
                int targetPinIndex,
                string signal)
            {
                var edge = new LayoutEdge(source, target, targetPinIndex, signal);
                edges.Add(edge);
                source.AddOutgoing(edge);
                target.AddIncoming(edge);
            }

            private void AssignInitialSlots()
            {
                foreach (var layer in nodes.GroupBy(static node => node.Layer))
                {
                    AssignSlots(layer.OrderBy(static node => node.InitialOrderKey));
                }
            }

            private void ReduceCrossings()
            {
                var maxGateLayer = nodes
                    .Where(static node => node.Role == NodeRole.Gate)
                    .Select(static node => node.Layer)
                    .DefaultIfEmpty(0)
                    .Max();

                for (var sweep = 0; sweep < SweepCount; sweep++)
                {
                    for (var layer = 1; layer <= maxGateLayer; layer++)
                    {
                        ReorderLayer(layer, useIncoming: true);
                    }

                    for (var layer = maxGateLayer; layer >= 1; layer--)
                    {
                        ReorderLayer(layer, useIncoming: false);
                    }
                }

                foreach (var node in nodes)
                {
                    node.OrderCommutativeInputs();
                }
            }

            private void ReorderLayer(
                int layer,
                bool useIncoming)
            {
                var layerNodes = nodes
                    .Where(node => node.Layer == layer && node.Role != NodeRole.Output)
                    .ToArray();
                if (layerNodes.Length < 2)
                {
                    foreach (var node in layerNodes)
                    {
                        node.OrderCommutativeInputs();
                    }

                    return;
                }

                AssignSlots(layerNodes
                    .OrderBy(node => node.GetBarycenter(useIncoming))
                    .ThenBy(static node => node.Slot)
                    .ThenBy(static node => node.InitialOrderKey));
                foreach (var node in layerNodes)
                {
                    node.OrderCommutativeInputs();
                }
            }

            private static void AssignSlots(IEnumerable<LayoutNode> orderedNodes)
            {
                var slot = 0;
                foreach (var node in orderedNodes)
                {
                    node.PlaceInSlot(slot++);
                }
            }

            private void AssignCoordinates()
            {
                foreach (var node in nodes.Where(static node => node.Role != NodeRole.Output))
                {
                    node.PlaceAt(
                        node.Kind == GatePaletteKind.Input
                            ? InputX
                            : Snap(gateX + Math.Max(0, node.Layer - 1) * levelSpacing),
                        Snap(40 + node.Slot * RowSpacing));
                }

                var outputX = Snap(gateX + maxSignalLevel * levelSpacing);
                foreach (var node in nodes
                    .Where(static node => node.Role == NodeRole.Output)
                    .OrderBy(static node => node.OriginalIndex))
                {
                    node.PlaceAt(
                        outputX,
                        node.TryGetSingleInputDriverY(out var driverY)
                            ? Snap(driverY)
                            : Snap(40 + node.OriginalIndex * RowSpacing));
                }
            }

            private void AssignHorizontalSpacing()
            {
                var maxBoundaryWireCount = CountBoundaryWires()
                    .DefaultIfEmpty(0)
                    .Max();
                var minimumColumnGap = GateOutputOffsetX + (maxBoundaryWireCount + 1) * RoutingGridStep;
                levelSpacing = Snap(Math.Max(LevelSpacing, minimumColumnGap));

                var firstBoundaryWireCount = edges
                    .Count(static edge => edge.Source.Layer == 0 && edge.Target.Layer > 0);
                var minimumInputGap = InputX + InputOutputOffsetX + (firstBoundaryWireCount + 1) * RoutingGridStep;
                gateX = Snap(Math.Max(GateX, minimumInputGap));
            }

            private IEnumerable<int> CountBoundaryWires()
            {
                for (var boundary = 1; boundary <= maxSignalLevel; boundary++)
                {
                    yield return edges.Count(edge => edge.Source.Layer <= boundary && edge.Target.Layer > boundary);
                }
            }

            private Dictionary<LayoutEdge, IReadOnlyList<GateDiagramWirePoint>> RouteEdges()
            {
                var boundaryLaneCounts = new Dictionary<int, int>();
                var routedEdges = new Dictionary<LayoutEdge, IReadOnlyList<GateDiagramWirePoint>>();
                var routeIndex = 0;
                var bottomRouteY = Snap(nodes
                    .Select(static node => node.Y + RowSpacing)
                    .DefaultIfEmpty(120)
                    .Max());

                foreach (var edge in edges
                    .OrderByDescending(static edge => edge.LayerSpan)
                    .ThenBy(static edge => edge.Source.Layer)
                    .ThenBy(static edge => edge.Source.Slot)
                    .ThenBy(static edge => edge.Target.Layer)
                    .ThenBy(static edge => edge.Target.Slot)
                    .ThenBy(static edge => edge.TargetPinIndex)
                    .ThenBy(static edge => edge.Signal, StringComparer.Ordinal))
                {
                    var routeY = bottomRouteY + routeIndex * RoutingGridStep;
                    var route = RouteEdge(edge, routeY, boundaryLaneCounts);
                    routedEdges[edge] = route;
                    routeIndex++;
                }

                return routedEdges;
            }

            private IReadOnlyList<GateDiagramWirePoint> RouteEdge(
                LayoutEdge edge,
                double routeY,
                IDictionary<int, int> boundaryLaneCounts)
            {
                var start = GetOutputConnection(edge.Source);
                var end = GetInputConnection(edge.Target, edge.TargetPinIndex);
                var route = new List<GateDiagramWirePoint>();
                var edgeChannelXs = new Dictionary<int, double>();

                var firstChannelX = GetBoundaryChannelX(edge.Source.Layer, boundaryLaneCounts, edgeChannelXs);
                AddPoint(route, firstChannelX, start.Y);
                AddPoint(route, firstChannelX, routeY);

                for (var boundary = edge.Source.Layer + 1; boundary < edge.Target.Layer; boundary++)
                {
                    AddPoint(route, GetBoundaryChannelX(boundary, boundaryLaneCounts, edgeChannelXs), routeY);
                }

                var lastChannelX = GetBoundaryChannelX(edge.Target.Layer - 1, boundaryLaneCounts, edgeChannelXs);
                AddPoint(route, lastChannelX, routeY);
                AddPoint(route, lastChannelX, end.Y);

                return route;
            }

            private double GetBoundaryChannelX(
                int boundary,
                IDictionary<int, int> boundaryLaneCounts,
                IDictionary<int, double> edgeChannelXs)
            {
                if (edgeChannelXs.TryGetValue(boundary, out var channelX))
                {
                    return channelX;
                }

                var lane = boundaryLaneCounts.TryGetValue(boundary, out var count)
                    ? count
                    : 0;
                boundaryLaneCounts[boundary] = lane + 1;
                channelX = Snap(GetLayerOutputX(boundary) + RoutingGridStep + lane * RoutingGridStep);
                edgeChannelXs[boundary] = channelX;
                return channelX;
            }

            private static void AddPoint(
                ICollection<GateDiagramWirePoint> route,
                double x,
                double y)
            {
                if (route.Count > 0 &&
                    route.Last() is { } last &&
                    Math.Abs(last.X - x) < 0.001 &&
                    Math.Abs(last.Y - y) < 0.001)
                {
                    return;
                }

                route.Add(new GateDiagramWirePoint(x, y));
            }

            private double GetLayerOutputX(int layer)
            {
                return layer == 0
                    ? InputX + InputOutputOffsetX
                    : gateX + Math.Max(0, layer - 1) * levelSpacing + GateOutputOffsetX;
            }

            private static GateDiagramWirePoint GetOutputConnection(LayoutNode node)
            {
                var offset = node.Kind switch
                {
                    GatePaletteKind.Input => InputOutputOffsetX,
                    GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne => ConstantOutputOffsetX,
                    _ => GateOutputOffsetX
                };
                return new GateDiagramWirePoint(node.X + offset, node.Y + PinCenterOffsetY);
            }

            private static GateDiagramWirePoint GetInputConnection(
                LayoutNode node,
                int pinIndex)
            {
                return new GateDiagramWirePoint(node.X, node.Y + GetInputOffsetY(node.Kind, node.InputCount, pinIndex));
            }

            private static double GetInputOffsetY(
                GatePaletteKind kind,
                int inputCount,
                int pinIndex)
            {
                if (kind is GatePaletteKind.Not or GatePaletteKind.Output)
                {
                    return PinCenterOffsetY;
                }

                if (kind == GatePaletteKind.Mux)
                {
                    return pinIndex switch
                    {
                        0 => 10,
                        1 => 25,
                        _ => 40
                    };
                }

                return 10 + pinIndex * GetInputOffset(inputCount);
            }

            private static double GetInputOffset(int inputCount)
            {
                return inputCount switch
                {
                    2 => 30,
                    3 => 15,
                    4 => 10,
                    _ => 30
                };
            }

            private IOrderedEnumerable<LayoutNode> OrderedNodes()
            {
                return nodes
                    .OrderBy(static node => node.Layer)
                    .ThenBy(static node => node.Slot)
                    .ThenBy(static node => node.OriginalIndex);
            }
        }

        private sealed class LayoutNode(
            string stableId,
            GatePaletteKind kind,
            int inputCount,
            string label,
            int originalIndex,
            int layer,
            NodeRole role)
        {
            private readonly List<LayoutEdge> incoming = [];

            private readonly List<LayoutEdge> outgoing = [];

            public string StableId { get; } = stableId;

            public GatePaletteKind Kind { get; } = kind;

            public int InputCount { get; } = inputCount;

            public string Label { get; } = label;

            public int OriginalIndex { get; } = originalIndex;

            public int Layer { get; } = layer;

            public NodeRole Role { get; } = role;

            public LayoutOrderKey InitialOrderKey { get; } = new(originalIndex, stableId);

            public bool NeedsComponentLabel
            {
                get
                {
                    if (Role != NodeRole.Gate)
                    {
                        return false;
                    }

                    return Kind is not GatePaletteKind.ConstantZero and not GatePaletteKind.ConstantOne;
                }
            }

            public int Slot { get; private set; }

            public double X { get; private set; }

            public double Y { get; private set; }

            public void AddIncoming(LayoutEdge edge) => incoming.Add(edge);

            public void AddOutgoing(LayoutEdge edge) => outgoing.Add(edge);

            public void PlaceInSlot(int slot) => Slot = slot;

            public void PlaceAt(
                double x,
                double y)
            {
                X = x;
                Y = y;
            }

            public double GetBarycenter(bool useIncoming)
            {
                var slots = (useIncoming
                        ? incoming.Select(static edge => edge.Source.Slot)
                        : outgoing.Select(static edge => edge.Target.Slot))
                    .ToArray();
                return slots.Length == 0
                    ? Slot
                    : slots.Average();
            }

            public void OrderCommutativeInputs()
            {
                if (!IsCommutative(Kind) || incoming.Count < 2)
                {
                    return;
                }

                var pinIndex = 0;
                foreach (var edge in incoming
                    .OrderBy(static edge => edge.Source.Slot)
                    .ThenBy(static edge => edge.Source.InitialOrderKey)
                    .ThenBy(static edge => edge.Signal, StringComparer.Ordinal))
                {
                    edge.AssignTargetPin(pinIndex++);
                }
            }

            public bool TryGetSingleInputDriverY(out double y)
            {
                if (incoming.Count == 0)
                {
                    y = 0;
                    return false;
                }

                y = incoming[0].Source.Y;
                return true;
            }
        }

        private sealed class LayoutEdge(
            LayoutNode source,
            LayoutNode target,
            int targetPinIndex,
            string signal)
        {
            public LayoutNode Source { get; } = source;

            public LayoutNode Target { get; } = target;

            public int TargetPinIndex { get; private set; } = targetPinIndex;

            public string Signal { get; } = signal;

            public int LayerSpan => Math.Max(1, Target.Layer - Source.Layer);

            public void AssignTargetPin(int pinIndex) => TargetPinIndex = pinIndex;
        }

        private readonly record struct LayoutOrderKey(
            int OriginalIndex,
            string StableId) : IComparable<LayoutOrderKey>
        {
            public int CompareTo(LayoutOrderKey other)
            {
                var indexComparison = OriginalIndex.CompareTo(other.OriginalIndex);
                return indexComparison != 0
                    ? indexComparison
                    : string.Compare(StableId, other.StableId, StringComparison.Ordinal);
            }
        }

        private readonly record struct MappedGateEntry(
            SisMappedGate Gate,
            int Index);

        private enum NodeRole
        {
            Input,

            Gate,

            Output
        }

        private static int GetInputCount(
            GatePaletteKind kind,
            int mappedInputCount)
        {
            return kind switch
            {
                GatePaletteKind.Not => 1,
                GatePaletteKind.Mux => 3,
                GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne => 0,
                _ => Math.Max(2, mappedInputCount)
            };
        }

        private static bool IsCommutative(GatePaletteKind kind)
        {
            return kind is
                GatePaletteKind.And or
                GatePaletteKind.Or or
                GatePaletteKind.Nand or
                GatePaletteKind.Nor;
        }

        public sealed record LayoutResult(
            IReadOnlyList<GateDiagramItem> Items,
            IReadOnlyList<GateDiagramWire> Wires);
    }

    private static string BuildMappedEquationText(LogicFunction source)
    {
        var sections = new List<string>();
        AddSection(sections, source.EquationText);
        if (source.MinimizedFunction is { } minimizedFunction)
        {
            AddSection(sections, minimizedFunction.EquationText);
        }

        return string.Join(Environment.NewLine + Environment.NewLine, sections);
    }

    private static void AddSection(ICollection<string> sections, string text)
    {
        if (!string.IsNullOrWhiteSpace(text))
        {
            sections.Add(text.TrimEnd());
        }
    }

    private static Dictionary<string, int> BuildSignalLevels(LogicFunction source, SisMappedNetwork mapped)
    {
        var signalLevels = source.InputNames.ToDictionary(
            static input => input,
            static _ => 0,
            StringComparer.Ordinal);

        var gatesByOutput = mapped.Gates
            .GroupBy(static gate => gate.Output, StringComparer.Ordinal)
            .ToDictionary(static group => group.Key, static group => group.First(), StringComparer.Ordinal);
        var visiting = new HashSet<string>(StringComparer.Ordinal);
        foreach (var gate in mapped.Gates)
        {
            ComputeSignalLevel(gate.Output, gatesByOutput, signalLevels, visiting);
        }

        return signalLevels;
    }

    private static Dictionary<string, string> BuildBufferAliases(SisMappedNetwork mapped)
    {
        var aliases = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var gate in mapped.Gates)
        {
            if (gate.Kind.Equals("buf", StringComparison.OrdinalIgnoreCase) &&
                gate.Inputs.Count > 0)
            {
                aliases[gate.Output] = gate.Inputs[0];
            }
        }

        return aliases;
    }

    private static string ResolveBufferAlias(
        string signal,
        IReadOnlyDictionary<string, string> bufferAliases)
    {
        var visited = new HashSet<string>(StringComparer.Ordinal);
        while (bufferAliases.TryGetValue(signal, out var aliasedSignal) && visited.Add(signal))
        {
            signal = aliasedSignal;
        }

        return signal;
    }

    private static int ComputeSignalLevel(
        string signal,
        IReadOnlyDictionary<string, SisMappedGate> gatesByOutput,
        Dictionary<string, int> signalLevels,
        HashSet<string> visiting)
    {
        if (signalLevels.TryGetValue(signal, out var level))
        {
            return level;
        }

        if (!gatesByOutput.TryGetValue(signal, out var gate) || !visiting.Add(signal))
        {
            return 0;
        }

        if (gate.Kind.Equals("buf", StringComparison.OrdinalIgnoreCase) && gate.Inputs.Count > 0)
        {
            level = ComputeSignalLevel(gate.Inputs[0], gatesByOutput, signalLevels, visiting);
        }
        else
        {
            level = Math.Max(
                1,
                gate.Inputs
                    .Select(input => ComputeSignalLevel(input, gatesByOutput, signalLevels, visiting))
                    .DefaultIfEmpty(0)
                    .Max()
                    + 1);
        }

        visiting.Remove(signal);
        signalLevels[signal] = level;
        return level;
    }

    private static GatePaletteKind ToGatePaletteKind(string kind)
    {
        var normalizedKind = kind.ToLowerInvariant();
        if (normalizedKind.StartsWith("nand", StringComparison.Ordinal))
        {
            return GatePaletteKind.Nand;
        }

        if (normalizedKind.StartsWith("nor", StringComparison.Ordinal))
        {
            return GatePaletteKind.Nor;
        }

        if (normalizedKind is "inv" or "inverter")
        {
            return GatePaletteKind.Not;
        }

        return normalizedKind switch
        {
            "and" => GatePaletteKind.And,
            "not" => GatePaletteKind.Not,
            "or" => GatePaletteKind.Or,
            "exo" or "xor" or "xor2" => GatePaletteKind.Xor,
            "mux" or "mux2" => GatePaletteKind.Mux,
            "zer" or "zero" or "zero0" => GatePaletteKind.ConstantZero,
            "one" or "one0" => GatePaletteKind.ConstantOne,
            "const0" => GatePaletteKind.ConstantZero,
            "const1" => GatePaletteKind.ConstantOne,
            _ => throw new SisMappingException($"Native SIS mapper returned unsupported gate kind '{kind}'.")
        };
    }

    private static double Snap(double value) => Math.Round(value / 20d) * 20d;

    private static string FormatInputPattern(int term, int inputCount)
    {
        var builder = new StringBuilder(inputCount);
        for (var inputIndex = 0; inputIndex < inputCount; inputIndex++)
        {
            var bitOffset = inputCount - inputIndex - 1;
            builder.Append(((term >> bitOffset) & 1) == 0 ? '0' : '1');
        }

        return builder.ToString();
    }
}
