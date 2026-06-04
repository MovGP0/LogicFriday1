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
        var items = new List<GateDiagramItem>();
        var wires = new List<GateDiagramWire>();
        var signalOutputs = new Dictionary<string, GateDiagramConnectionReference>(StringComparer.Ordinal);
        var signalLevels = BuildSignalLevels(source, mapped);
        var signalRows = new Dictionary<string, double>(StringComparer.Ordinal);
        var nextItemId = 1;

        for (var index = 0; index < source.InputNames.Length; index++)
        {
            var item = new GateDiagramItem(
                GatePaletteKind.Input,
                0,
                InputX,
                40 + index * RowSpacing,
                source.InputNames[index],
                Id: nextItemId++);
            items.Add(item);
            signalOutputs[item.Label] = OutputOf(item);
            signalRows[item.Label] = item.Y;
        }

        var levelRows = new Dictionary<int, int>();
        var componentNumber = 1;
        foreach (var mappedGate in mapped.Gates)
        {
            if (mappedGate.Kind.Equals("buf", StringComparison.OrdinalIgnoreCase))
            {
                if (mappedGate.Inputs.Count > 0 &&
                    signalOutputs.TryGetValue(mappedGate.Inputs[0], out var sourceReference))
                {
                    signalOutputs[mappedGate.Output] = sourceReference;
                }

                continue;
            }

            var kind = ToGatePaletteKind(mappedGate.Kind);
            var level = signalLevels.GetValueOrDefault(mappedGate.Output, Math.Max(1, mappedGate.Level));
            var row = levelRows.GetValueOrDefault(level);
            levelRows[level] = row + 1;
            var inputCount = kind switch
            {
                GatePaletteKind.Not => 1,
                GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne => 0,
                _ => Math.Max(2, mappedGate.Inputs.Count)
            };
            var item = new GateDiagramItem(
                kind,
                inputCount,
                Snap(GateX + Math.Max(0, level - 1) * LevelSpacing),
                40 + row * RowSpacing,
                "",
                kind is GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne
                    ? string.Empty
                    : $"[{componentNumber++}]",
                nextItemId++);
            items.Add(item);

            for (var inputIndex = 0; inputIndex < mappedGate.Inputs.Count; inputIndex++)
            {
                if (signalOutputs.TryGetValue(mappedGate.Inputs[inputIndex], out var sourceReference))
                {
                    wires.Add(new GateDiagramWire(
                        sourceReference,
                        new GateDiagramConnectionReference(
                            item.Id,
                            GateDiagramConnectionKind.Input,
                            inputIndex)));
                }
            }

            signalOutputs[mappedGate.Output] = OutputOf(item);
            signalRows[mappedGate.Output] = item.Y;
        }

        var maxLevel = Math.Max(1, signalLevels.Values.DefaultIfEmpty(1).Max());
        var outputX = Snap(GateX + (maxLevel + 1) * LevelSpacing);
        for (var index = 0; index < source.OutputNames.Length; index++)
        {
            var outputY = signalRows.TryGetValue(source.OutputNames[index], out var driverY)
                ? Snap(driverY)
                : 40 + index * RowSpacing;
            var item = new GateDiagramItem(
                GatePaletteKind.Output,
                1,
                outputX,
                outputY,
                source.OutputNames[index],
                Id: nextItemId++);
            items.Add(item);
            if (signalOutputs.TryGetValue(source.OutputNames[index], out var sourceReference))
            {
                wires.Add(new GateDiagramWire(
                    sourceReference,
                    new GateDiagramConnectionReference(
                        item.Id,
                        GateDiagramConnectionKind.Input,
                        0)));
            }
        }

        return new GateDiagramFunction(
            [.. source.InputNames],
            [.. source.OutputNames],
            source.OutputValues.Select(static row => row.ToArray()).ToArray(),
            "Mapped to gates:" + Environment.NewLine + source.EquationText,
            items,
            wires,
            source.MinimizedFunction);
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
        return kind.ToLowerInvariant() switch
        {
            "and" => GatePaletteKind.And,
            "not" => GatePaletteKind.Not,
            "or" => GatePaletteKind.Or,
            "const0" => GatePaletteKind.ConstantZero,
            "const1" => GatePaletteKind.ConstantOne,
            _ => throw new SisMappingException($"Native SIS mapper returned unsupported gate kind '{kind}'.")
        };
    }

    private static GateDiagramConnectionReference OutputOf(GateDiagramItem item) =>
        new(item.Id, GateDiagramConnectionKind.Output, 0);

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
