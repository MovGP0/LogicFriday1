using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public sealed class LogicFunctionFileService : ILogicFunctionFileService
{
    private const string Header = "LTK1-LOGICFRIDAY1-PORT-LFCN";

    private const int CurrentVersion = 1;

    private static readonly JsonSerializerOptions s_jsonOptions = new()
    {
        WriteIndented = true,
        Converters =
        {
            new JsonStringEnumConverter()
        }
    };

    public LogicFunction Load(string filePath)
    {
        try
        {
            var text = File.ReadAllText(filePath, Encoding.UTF8);
            if (!text.StartsWith(Header, StringComparison.Ordinal))
            {
                throw new LogicFunctionFileException("The file is not a LogicFriday1 port logic function file.");
            }

            var json = text[Header.Length..].TrimStart();
            var file = JsonSerializer.Deserialize<LogicFunctionFileDto>(json, s_jsonOptions);
            if (file is null)
            {
                throw new LogicFunctionFileException("The logic function file is empty.");
            }

            if (file.Version > CurrentVersion)
            {
                throw new LogicFunctionFileException("The logic function file version is newer than this application supports.");
            }

            return FromDto(file.Function);
        }
        catch (LogicFunctionFileException)
        {
            throw;
        }
        catch (Exception ex)
        {
            throw new LogicFunctionFileException("The logic function file could not be loaded.", ex);
        }
    }

    public void Save(string filePath, LogicFunction logicFunction)
    {
        try
        {
            var file = new LogicFunctionFileDto(CurrentVersion, ToDto(logicFunction));
            var json = JsonSerializer.Serialize(file, s_jsonOptions);
            File.WriteAllText(filePath, $"{Header}{Environment.NewLine}{json}", Encoding.UTF8);
        }
        catch (Exception ex)
        {
            throw new LogicFunctionFileException("The logic function file could not be saved.", ex);
        }
    }

    private static LogicFunctionDto ToDto(LogicFunction logicFunction)
    {
        var functionKind = logicFunction switch
        {
            TruthTableLogicFunction => LogicFunctionKind.TruthTable,
            LogicEquationFunction => LogicFunctionKind.LogicEquation,
            GateDiagramFunction => LogicFunctionKind.GateDiagram,
            _ => throw new LogicFunctionFileException("Unsupported logic function type.")
        };

        var gateDiagramFunction = logicFunction as GateDiagramFunction;

        return new LogicFunctionDto(
            functionKind,
            logicFunction.InputNames.ToArray(),
            logicFunction.OutputNames.ToArray(),
            logicFunction.OutputValues.Select(static row => row.ToArray()).ToArray(),
            logicFunction.EquationText,
            ToDto(logicFunction.MinimizedFunction),
            gateDiagramFunction?.Items.ToArray(),
            gateDiagramFunction?.Wires.ToArray());
    }

    private static MinimizedLogicFunctionDto? ToDto(MinimizedLogicFunction? minimizedFunction)
    {
        if (minimizedFunction is null)
        {
            return null;
        }

        return new MinimizedLogicFunctionDto(
            minimizedFunction.Products
                .Select(static product => new MinimizedProductTermDto(
                    product.InputPattern,
                    product.OutputValues.ToArray()))
                .ToArray(),
            minimizedFunction.EquationText,
            minimizedFunction.PlaText);
    }

    private static LogicFunction FromDto(LogicFunctionDto function)
    {
        Validate(function);

        var minimizedFunction = FromDto(function.MinimizedFunction);
        return function.Kind switch
        {
            LogicFunctionKind.TruthTable => new TruthTableLogicFunction(
                function.InputNames,
                function.OutputNames,
                function.OutputValues,
                function.EquationText,
                minimizedFunction),
            LogicFunctionKind.LogicEquation => new LogicEquationFunction(
                function.InputNames,
                function.OutputNames,
                function.OutputValues,
                function.EquationText,
                minimizedFunction),
            LogicFunctionKind.GateDiagram => new GateDiagramFunction(
                function.InputNames,
                function.OutputNames,
                function.OutputValues,
                function.EquationText,
                function.Items ?? [],
                function.Wires ?? [],
                minimizedFunction),
            _ => throw new LogicFunctionFileException("Unsupported logic function type.")
        };
    }

    private static MinimizedLogicFunction? FromDto(MinimizedLogicFunctionDto? minimizedFunction)
    {
        if (minimizedFunction is null)
        {
            return null;
        }

        return new MinimizedLogicFunction(
            minimizedFunction.Products
                .Select(static product => new MinimizedProductTerm(
                    product.InputPattern,
                    product.OutputValues))
                .ToArray(),
            minimizedFunction.EquationText,
            minimizedFunction.PlaText);
    }

    private static void Validate(LogicFunctionDto function)
    {
        if (function.InputNames.Length > 30)
        {
            throw new LogicFunctionFileException("The logic function has too many inputs.");
        }

        var expectedRows = 1 << function.InputNames.Length;
        if (function.OutputValues.Length != expectedRows)
        {
            throw new LogicFunctionFileException("The logic function truth table row count is invalid.");
        }

        foreach (var outputValues in function.OutputValues)
        {
            if (outputValues.Length != function.OutputNames.Length)
            {
                throw new LogicFunctionFileException("The logic function truth table output count is invalid.");
            }

            foreach (var outputValue in outputValues)
            {
                if (outputValue is not ("0" or "1" or "X"))
                {
                    throw new LogicFunctionFileException("The logic function truth table contains an invalid output value.");
                }
            }
        }
    }

    private sealed record LogicFunctionFileDto(
        int Version,
        LogicFunctionDto Function);

    private sealed record LogicFunctionDto(
        LogicFunctionKind Kind,
        string[] InputNames,
        string[] OutputNames,
        string[][] OutputValues,
        string EquationText,
        MinimizedLogicFunctionDto? MinimizedFunction,
        GateDiagramItem[]? Items,
        GateDiagramWire[]? Wires);

    private sealed record MinimizedLogicFunctionDto(
        MinimizedProductTermDto[] Products,
        string EquationText,
        string PlaText);

    private sealed record MinimizedProductTermDto(
        string InputPattern,
        string[] OutputValues);

    private enum LogicFunctionKind
    {
        TruthTable,
        LogicEquation,
        GateDiagram
    }
}
