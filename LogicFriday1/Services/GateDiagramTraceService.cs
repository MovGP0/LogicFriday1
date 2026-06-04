using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class GateDiagramTraceService
{
    public static GateDiagramTraceResult Evaluate(
        GateDiagramFunction gateDiagramFunction,
        IReadOnlyDictionary<string, int> inputValues)
    {
        var itemById = gateDiagramFunction.Items.ToDictionary(static item => item.Id);
        var driverByInput = ResolveInputDrivers(gateDiagramFunction.Items, gateDiagramFunction.Wires, itemById);
        var itemValues = new Dictionary<int, int>();
        var connectionValues = new Dictionary<GateDiagramConnectionReference, int>();
        var outputValues = new Dictionary<string, int>(StringComparer.Ordinal);

        foreach (var item in gateDiagramFunction.Items)
        {
            if (item.Kind == GatePaletteKind.Input)
            {
                var value = inputValues.TryGetValue(item.Label, out var inputValue) && inputValue != 0
                    ? 1
                    : 0;
                itemValues[item.Id] = value;
                connectionValues[new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Output, 0)] = value;
            }
        }

        foreach (var output in gateDiagramFunction.Items.Where(static item => item.Kind == GatePaletteKind.Output))
        {
            var input = new GateDiagramConnectionReference(output.Id, GateDiagramConnectionKind.Input, 0);
            var value = driverByInput.TryGetValue(input, out var driver)
                ? EvaluateDriver(driver, itemById, driverByInput, itemValues, connectionValues)
                : 0;
            itemValues[output.Id] = value;
            connectionValues[input] = value;
            outputValues[output.Label] = value;
        }

        foreach (var item in gateDiagramFunction.Items.Where(static item => HasOutputConnection(item.Kind)))
        {
            var output = new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Output, 0);
            if (!connectionValues.ContainsKey(output))
            {
                connectionValues[output] = EvaluateDriver(output, itemById, driverByInput, itemValues, connectionValues);
            }
        }

        foreach (var input in driverByInput.Keys)
        {
            if (!connectionValues.ContainsKey(input))
            {
                connectionValues[input] = EvaluateDriver(driverByInput[input], itemById, driverByInput, itemValues, connectionValues);
            }
        }

        return new GateDiagramTraceResult(itemValues, connectionValues, outputValues);
    }

    private static Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> ResolveInputDrivers(
        IReadOnlyList<GateDiagramItem> items,
        IReadOnlyList<GateDiagramWire> wires,
        IReadOnlyDictionary<int, GateDiagramItem> itemById)
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
            if (parent.ContainsKey(wire.Start) &&
                parent.ContainsKey(wire.End))
            {
                Union(wire.Start, wire.End, parent);
            }
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

        var driverByInput = new Dictionary<GateDiagramConnectionReference, GateDiagramConnectionReference>();
        foreach (var net in nets.Values)
        {
            var driver = net.FirstOrDefault(static connection => connection.Kind == GateDiagramConnectionKind.Output);
            if (driver == default)
            {
                continue;
            }

            foreach (var input in net.Where(static connection => connection.Kind == GateDiagramConnectionKind.Input))
            {
                if (itemById.ContainsKey(input.ItemId))
                {
                    driverByInput[input] = driver;
                }
            }
        }

        return driverByInput;
    }

    private static int EvaluateDriver(
        GateDiagramConnectionReference driver,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        IDictionary<int, int> itemValues,
        IDictionary<GateDiagramConnectionReference, int> connectionValues)
    {
        if (connectionValues.TryGetValue(driver, out var connectionValue))
        {
            return connectionValue;
        }

        var item = itemById[driver.ItemId];
        if (itemValues.TryGetValue(item.Id, out var itemValue))
        {
            connectionValues[driver] = itemValue;
            return itemValue;
        }

        var value = item.Kind switch
        {
            GatePaletteKind.ConstantZero => 0,
            GatePaletteKind.ConstantOne => 1,
            GatePaletteKind.Not => 1 - EvaluateInput(item, 0, itemById, driverByInput, itemValues, connectionValues),
            GatePaletteKind.Nand => 1 - EvaluateInputs(item, itemById, driverByInput, itemValues, connectionValues).Min(),
            GatePaletteKind.And => EvaluateInputs(item, itemById, driverByInput, itemValues, connectionValues).Min(),
            GatePaletteKind.Nor => 1 - EvaluateInputs(item, itemById, driverByInput, itemValues, connectionValues).Max(),
            GatePaletteKind.Or => EvaluateInputs(item, itemById, driverByInput, itemValues, connectionValues).Max(),
            GatePaletteKind.Xor => EvaluateInputs(item, itemById, driverByInput, itemValues, connectionValues).Sum() % 2,
            GatePaletteKind.Mux => EvaluateMux(item, itemById, driverByInput, itemValues, connectionValues),
            GatePaletteKind.Input => itemValues.TryGetValue(item.Id, out var inputValue) ? inputValue : 0,
            _ => 0
        };

        itemValues[item.Id] = value;
        connectionValues[driver] = value;
        return value;
    }

    private static int EvaluateMux(
        GateDiagramItem item,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        IDictionary<int, int> itemValues,
        IDictionary<GateDiagramConnectionReference, int> connectionValues)
    {
        var d0 = EvaluateInput(item, 0, itemById, driverByInput, itemValues, connectionValues);
        var d1 = EvaluateInput(item, 1, itemById, driverByInput, itemValues, connectionValues);
        var selector = EvaluateInput(item, 2, itemById, driverByInput, itemValues, connectionValues);
        return selector == 0 ? d0 : d1;
    }

    private static IEnumerable<int> EvaluateInputs(
        GateDiagramItem item,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        IDictionary<int, int> itemValues,
        IDictionary<GateDiagramConnectionReference, int> connectionValues)
    {
        for (var inputIndex = 0; inputIndex < item.InputCount; inputIndex++)
        {
            yield return EvaluateInput(item, inputIndex, itemById, driverByInput, itemValues, connectionValues);
        }
    }

    private static int EvaluateInput(
        GateDiagramItem item,
        int pinIndex,
        IReadOnlyDictionary<int, GateDiagramItem> itemById,
        IReadOnlyDictionary<GateDiagramConnectionReference, GateDiagramConnectionReference> driverByInput,
        IDictionary<int, int> itemValues,
        IDictionary<GateDiagramConnectionReference, int> connectionValues)
    {
        var input = new GateDiagramConnectionReference(item.Id, GateDiagramConnectionKind.Input, pinIndex);
        return driverByInput.TryGetValue(input, out var driver)
            ? EvaluateDriver(driver, itemById, driverByInput, itemValues, connectionValues)
            : 0;
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
