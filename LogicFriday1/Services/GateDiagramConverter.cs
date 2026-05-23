using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class GateDiagramConverter
{
    public static GateDiagramConversionResult Convert(
        IReadOnlyList<GateDiagramItem> items,
        IReadOnlyList<GateDiagramWire> wires)
    {
        var itemById = items.ToDictionary(static item => item.Id);
        var inputs = items.Where(static item => item.Kind == GatePaletteKind.Input).ToArray();
        var outputs = items.Where(static item => item.Kind == GatePaletteKind.Output).ToArray();

        ValidateTerminalCounts(inputs.Length, outputs.Length);
        ValidateWireReferences(wires, itemById);

        var nets = BuildNets(items, wires);
        var driverByInput = ResolveInputDrivers(items, nets);
        ValidateGateOutputs(items, nets);
        ValidateInputTerminalsAreUsed(inputs, wires);
        ValidateNoFeedback(items, driverByInput);

        var inputNames = inputs.Select(static item => item.Label).ToArray();
        var outputNames = outputs.Select(static item => item.Label).ToArray();
        var outputValues = EvaluateTruthTable(inputs, outputs, driverByInput, itemById);

        return new GateDiagramConversionResult(inputNames, outputNames, outputValues);
    }

    private static void ValidateTerminalCounts(int inputCount, int outputCount)
    {
        if (inputCount < 2)
        {
            throw new GateDiagramConversionException("The function must have at least two inputs.");
        }

        if (inputCount > 16)
        {
            throw new GateDiagramConversionException("The number of inputs is limited to 16.");
        }

        if (outputCount == 0)
        {
            throw new GateDiagramConversionException("The function must have at least one output.");
        }

        if (outputCount > 16)
        {
            throw new GateDiagramConversionException("The number of outputs is limited to 16.");
        }
    }

    private static void ValidateWireReferences(
        IReadOnlyList<GateDiagramWire> wires,
        IReadOnlyDictionary<int, GateDiagramItem> itemById)
    {
        foreach (var wire in wires)
        {
            ValidateConnectionReference(wire.Start, itemById);
            ValidateConnectionReference(wire.End, itemById);
        }
    }

    private static void ValidateConnectionReference(
        GateDiagramConnectionReference reference,
        IReadOnlyDictionary<int, GateDiagramItem> itemById)
    {
        if (!itemById.TryGetValue(reference.ItemId, out var item) ||
            !HasConnection(item, reference))
        {
            throw new GateDiagramConversionException("Unknown gate diagram connection.");
        }
    }

    private static Dictionary<GateDiagramConnectionReference, List<GateDiagramConnectionReference>> BuildNets(
        IReadOnlyList<GateDiagramItem> items,
        IReadOnlyList<GateDiagramWire> wires)
    {
        var parent = new Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference>();
        foreach (var item in items)
        {
            foreach (var connection in EnumerateConnectionReferences(item))
            {
                parent[connection] = connection;
            }
        }

        foreach (var wire in wires)
        {
            Union(wire.Start, wire.End, parent);
        }

        var nets = new Dictionary<GateDiagramConnectionReference, List<GateDiagramConnectionReference>>();
        foreach (var connection in parent.Keys)
        {
            var root = Find(connection, parent);
            if (!nets.TryGetValue(root, out var connections))
            {
                connections = [];
                nets[root] = connections;
            }

            connections.Add(connection);
        }

        return nets;
    }

    private static Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> ResolveInputDrivers(
        IReadOnlyList<GateDiagramItem> items,
        Dictionary<GateDiagramConnectionReference, List<GateDiagramConnectionReference>> nets)
    {
        var driverByInput = new Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference>();
        foreach (var net in nets.Values)
        {
            var outputs = net
                .Where(static connection => connection.Kind == GateDiagramConnectionKind.Output)
                .ToArray();

            if (outputs.Length > 1)
            {
                throw new GateDiagramConversionException("Gate output is connected to another output.");
            }

            if (outputs.Length == 0)
            {
                continue;
            }

            var driver = outputs[0];
            foreach (var input in net.Where(static connection => connection.Kind == GateDiagramConnectionKind.Input))
            {
                driverByInput[input] = driver;
            }
        }

        foreach (var item in items)
        {
            foreach (var input in EnumerateInputReferences(item))
            {
                if (!driverByInput.ContainsKey(input))
                {
                    throw new GateDiagramConversionException(
                        item.Kind == GatePaletteKind.Output
                            ? $"Output terminal {item.Label} is not connected."
                            : $"Gate {GetGateName(item)}: Missing input connection.");
                }
            }
        }

        return driverByInput;
    }

    private static void ValidateGateOutputs(
        IReadOnlyList<GateDiagramItem> items,
        Dictionary<GateDiagramConnectionReference, List<GateDiagramConnectionReference>> nets)
    {
        var connectedOutputs = nets.Values
            .Where(static net => net.Count > 1)
            .SelectMany(static net => net)
            .Where(static connection => connection.Kind == GateDiagramConnectionKind.Output)
            .ToHashSet();

        foreach (var item in items)
        {
            if (!RequiresOutputConnection(item.Kind))
            {
                continue;
            }

            var output = new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Output, 0);
            if (!connectedOutputs.Contains(output))
            {
                throw new GateDiagramConversionException($"Gate {GetGateName(item)}: Missing output connection.");
            }
        }
    }

    private static void ValidateInputTerminalsAreUsed(
        IReadOnlyList<GateDiagramItem> inputs,
        IReadOnlyList<GateDiagramWire> wires)
    {
        var connected = wires
            .SelectMany(static wire => new[] { wire.Start, wire.End })
            .Where(static connection => connection.Kind == GateDiagramConnectionKind.Output)
            .Select(static connection => connection.ItemId)
            .ToHashSet();

        foreach (var input in inputs)
        {
            if (!connected.Contains(input.Id))
            {
                throw new GateDiagramConversionException($"Input terminal {input.Label} is not connected.");
            }
        }
    }

    private static void ValidateNoFeedback(
        IReadOnlyList<GateDiagramItem> items,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput)
    {
        var gateIds = items
            .Where(static item => IsLogicGate(item.Kind))
            .Select(static item => item.Id)
            .ToHashSet();

        var dependencies = gateIds.ToDictionary(static id => id, static _ => new List<int>());
        foreach (var item in items.Where(static item => IsLogicGate(item.Kind)))
        {
            foreach (var input in EnumerateInputReferences(item))
            {
                if (driverByInput.TryGetValue(input, out var driver) &&
                    gateIds.Contains(driver.ItemId))
                {
                    dependencies[item.Id].Add(driver.ItemId);
                }
            }
        }

        var visiting = new HashSet<int>();
        var visited = new HashSet<int>();
        foreach (var gateId in gateIds)
        {
            if (HasCycle(gateId, dependencies, visiting, visited))
            {
                throw new GateDiagramConversionException(
                    $"Gate {GetGateName(items.First(item => item.Id == gateId))}: An input is a function of the gate's output. Feedback is not supported.");
            }
        }
    }

    private static IReadOnlyList<string[]> EvaluateTruthTable(
        IReadOnlyList<GateDiagramItem> inputs,
        IReadOnlyList<GateDiagramItem> outputs,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        IReadOnlyDictionary<int, GateDiagramItem> itemById)
    {
        var inputIndexById = inputs
            .Select((item, index) => new
            {
                item.Id,
                Index = index
            })
            .ToDictionary(static item => item.Id, static item => item.Index);

        var rowCount = 1 << inputs.Count;
        var rows = new string[rowCount][];
        for (var term = 0; term < rowCount; term++)
        {
            var row = new string[outputs.Count];
            for (var outputIndex = 0; outputIndex < outputs.Count; outputIndex++)
            {
                var outputInput = new GateDiagramConnectionReference(
                    outputs[outputIndex].Id,
                    GateDiagramConnectionKind.Input,
                    0);

                row[outputIndex] = EvaluateDriver(
                    driverByInput[outputInput],
                    term,
                    inputIndexById,
                    itemById,
                    driverByInput,
                    []).ToString();
            }

            rows[term] = row;
        }

        return rows;
    }

    private static int EvaluateDriver(
        GateDiagramConnectionReference driver,
        int term,
        IReadOnlyDictionary<int, int> inputIndexById,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        Dictionary<int, int> valueCache)
    {
        if (valueCache.TryGetValue(driver.ItemId, out var cached))
        {
            return cached;
        }

        var item = itemById[driver.ItemId];
        var value = item.Kind switch
        {
            GatePaletteKind.Input => GetInputValue(term, inputIndexById[item.Id], inputIndexById.Count),
            GatePaletteKind.ConstantZero => 0,
            GatePaletteKind.ConstantOne => 1,
            GatePaletteKind.Not => 1 - EvaluateInput(item, 0, term, inputIndexById, itemById, driverByInput, valueCache),
            GatePaletteKind.Nand => 1 - EvaluateInputs(item, term, inputIndexById, itemById, driverByInput, valueCache).Min(),
            GatePaletteKind.And => EvaluateInputs(item, term, inputIndexById, itemById, driverByInput, valueCache).Min(),
            GatePaletteKind.Nor => 1 - EvaluateInputs(item, term, inputIndexById, itemById, driverByInput, valueCache).Max(),
            GatePaletteKind.Or => EvaluateInputs(item, term, inputIndexById, itemById, driverByInput, valueCache).Max(),
            GatePaletteKind.Xor => EvaluateInputs(item, term, inputIndexById, itemById, driverByInput, valueCache).Sum() % 2,
            GatePaletteKind.Mux => EvaluateMux(item, term, inputIndexById, itemById, driverByInput, valueCache),
            _ => throw new GateDiagramConversionException($"Gate {GetGateName(item)} cannot be evaluated.")
        };

        valueCache[item.Id] = value;
        return value;
    }

    private static int EvaluateMux(
        GateDiagramItem item,
        int term,
        IReadOnlyDictionary<int, int> inputIndexById,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        Dictionary<int, int> valueCache)
    {
        var d0 = EvaluateInput(item, 0, term, inputIndexById, itemById, driverByInput, valueCache);
        var d1 = EvaluateInput(item, 1, term, inputIndexById, itemById, driverByInput, valueCache);
        var selector = EvaluateInput(item, 2, term, inputIndexById, itemById, driverByInput, valueCache);
        return selector == 0 ? d0 : d1;
    }

    private static IEnumerable<int> EvaluateInputs(
        GateDiagramItem item,
        int term,
        IReadOnlyDictionary<int, int> inputIndexById,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        Dictionary<int, int> valueCache)
    {
        for (var inputIndex = 0; inputIndex < item.InputCount; inputIndex++)
        {
            yield return EvaluateInput(item, inputIndex, term, inputIndexById, itemById, driverByInput, valueCache);
        }
    }

    private static int EvaluateInput(
        GateDiagramItem item,
        int pinIndex,
        int term,
        IReadOnlyDictionary<int, int> inputIndexById,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        Dictionary<int, int> valueCache)
    {
        var input = new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Input, pinIndex);
        return EvaluateDriver(driverByInput[input], term, inputIndexById, itemById, driverByInput, valueCache);
    }

    private static int GetInputValue(int term, int inputIndex, int inputCount)
    {
        var bitOffset = inputCount - inputIndex - 1;
        return (term >> bitOffset) & 1;
    }

    private static bool HasCycle(
        int gateId,
        IReadOnlyDictionary<int, List<int>> dependencies,
        HashSet<int> visiting,
        HashSet<int> visited)
    {
        if (visited.Contains(gateId))
        {
            return false;
        }

        if (!visiting.Add(gateId))
        {
            return true;
        }

        foreach (var dependency in dependencies[gateId])
        {
            if (HasCycle(dependency, dependencies, visiting, visited))
            {
                return true;
            }
        }

        visiting.Remove(gateId);
        visited.Add(gateId);
        return false;
    }

    private static IEnumerable<GateDiagramConnectionReference> EnumerateConnectionReferences(GateDiagramItem item)
    {
        foreach (var input in EnumerateInputReferences(item))
        {
            yield return input;
        }

        if (HasOutputConnection(item.Kind))
        {
            yield return new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Output, 0);
        }
    }

    private static IEnumerable<GateDiagramConnectionReference> EnumerateInputReferences(GateDiagramItem item)
    {
        var inputCount = item.Kind switch
        {
            GatePaletteKind.Not => 1,
            GatePaletteKind.Nand or GatePaletteKind.And or GatePaletteKind.Nor or GatePaletteKind.Or or GatePaletteKind.Xor => item.InputCount,
            GatePaletteKind.Mux => 3,
            GatePaletteKind.Output => 1,
            _ => 0
        };

        for (var index = 0; index < inputCount; index++)
        {
            yield return new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Input, index);
        }
    }

    private static bool HasConnection(GateDiagramItem item, GateDiagramConnectionReference reference)
    {
        if (reference.Kind == GateDiagramConnectionKind.Output)
        {
            return reference.PinIndex == 0 && HasOutputConnection(item.Kind);
        }

        return EnumerateInputReferences(item).Any(input => input.PinIndex == reference.PinIndex);
    }

    private static bool HasOutputConnection(GatePaletteKind kind)
    {
        return kind is
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.Nor or
            GatePaletteKind.Mux or
            GatePaletteKind.And or
            GatePaletteKind.Or or
            GatePaletteKind.Xor or
            GatePaletteKind.ConstantZero or
            GatePaletteKind.ConstantOne or
            GatePaletteKind.Input;
    }

    private static bool RequiresOutputConnection(GatePaletteKind kind)
    {
        return kind is
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.Nor or
            GatePaletteKind.Mux or
            GatePaletteKind.And or
            GatePaletteKind.Or or
            GatePaletteKind.Xor or
            GatePaletteKind.ConstantZero or
            GatePaletteKind.ConstantOne;
    }

    private static bool IsLogicGate(GatePaletteKind kind)
    {
        return kind is
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.Nor or
            GatePaletteKind.Mux or
            GatePaletteKind.And or
            GatePaletteKind.Or or
            GatePaletteKind.Xor;
    }

    private static string GetGateName(GateDiagramItem item)
    {
        return item.ComponentLabel.Length == 0 ? item.Label : item.ComponentLabel;
    }

    private static GateDiagramConnectionReference Find(
        GateDiagramConnectionReference connection,
        Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> parent)
    {
        var root = parent[connection];
        if (root == connection)
        {
            return root;
        }

        root = Find(root, parent);
        parent[connection] = root;
        return root;
    }

    private static void Union(
        GateDiagramConnectionReference left,
        GateDiagramConnectionReference right,
        Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> parent)
    {
        var leftRoot = Find(left, parent);
        var rightRoot = Find(right, parent);
        if (leftRoot != rightRoot)
        {
            parent[rightRoot] = leftRoot;
        }
    }
}
