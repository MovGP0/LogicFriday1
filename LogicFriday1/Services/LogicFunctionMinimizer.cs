using System.Text;
using Espresso;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class LogicFunctionMinimizer
{
    public static MinimizedLogicFunction Minimize(LogicFunction logicFunction)
    {
        return Minimize(logicFunction, MinimizeOptions.Default);
    }

    public static MinimizedLogicFunction Minimize(LogicFunction logicFunction, MinimizeOptions options)
    {
        if (options.UseExactMode)
        {
            throw new NotSupportedException(
                "Exact minimization requires the native Espresso Rust port and is not available yet.");
        }

        return options.MinimizeOutputsIndependently && logicFunction.OutputNames.Length > 1
            ? MinimizeOutputsIndependently(logicFunction)
            : MinimizeJointOutputs(logicFunction);
    }

    private static MinimizedLogicFunction MinimizeJointOutputs(LogicFunction logicFunction)
    {
        var plaText = BuildPla(logicFunction);
        if (PlaReader.Read(new StringReader(plaText), true, out var pla) == -1 ||
            pla?.F is null ||
            pla.D is null ||
            pla.R is null)
        {
            throw new InvalidOperationException("The function could not be converted to Espresso PLA input.");
        }

        pla.F = EspressoMinimizer.Minimize(pla.Cube, pla.F, pla.D, pla.R);

        var minimizedPlaText = WritePla(pla, PlaData.FType);
        var products = ParseMinimizedProducts(minimizedPlaText, logicFunction.OutputNames.Length);
        var equationText = GenerateMinimizedEquation(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            products);

        return new MinimizedLogicFunction(products, equationText, minimizedPlaText);
    }

    private static MinimizedLogicFunction MinimizeOutputsIndependently(LogicFunction logicFunction)
    {
        var products = new List<MinimizedProductTerm>();
        for (var outputIndex = 0; outputIndex < logicFunction.OutputNames.Length; outputIndex++)
        {
            var singleOutputFunction = new TruthTableLogicFunction(
                logicFunction.InputNames,
                [logicFunction.OutputNames[outputIndex]],
                logicFunction.OutputValues
                    .Select(row => new[] { row[outputIndex] })
                    .ToArray(),
                logicFunction.EquationText);

            foreach (var product in MinimizeJointOutputs(singleOutputFunction).Products)
            {
                var outputValues = Enumerable.Repeat("0", logicFunction.OutputNames.Length).ToArray();
                outputValues[outputIndex] = product.OutputValues[0];
                products.Add(new MinimizedProductTerm(product.InputPattern, outputValues));
            }
        }

        var equationText = GenerateMinimizedEquation(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            products);
        var minimizedPlaText = BuildMinimizedPla(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            products);

        return new MinimizedLogicFunction(products, equationText, minimizedPlaText);
    }

    private static string BuildPla(LogicFunction logicFunction)
    {
        var builder = new StringBuilder();
        builder.AppendLine($".i {logicFunction.InputNames.Length}");
        builder.AppendLine($".o {logicFunction.OutputNames.Length}");
        builder.Append(".ilb");
        foreach (var inputName in logicFunction.InputNames)
        {
            builder.Append(' ').Append(inputName);
        }

        builder.AppendLine();
        builder.Append(".ob");
        foreach (var outputName in logicFunction.OutputNames)
        {
            builder.Append(' ').Append(outputName);
        }

        builder.AppendLine();
        builder.AppendLine(".type fd");

        var productCount = logicFunction.OutputValues.Count(row =>
            row.Any(static value => value is "1" or "X"));
        builder.AppendLine($".p {productCount}");

        for (var term = 0; term < logicFunction.OutputValues.Count; term++)
        {
            var outputs = logicFunction.OutputValues[term];
            if (outputs.All(static value => value is not ("1" or "X")))
            {
                continue;
            }

            builder
                .Append(FormatInputPattern(term, logicFunction.InputNames.Length))
                .Append(' ');

            foreach (var outputValue in outputs)
            {
                builder.Append(outputValue == "X" ? '-' : outputValue);
            }

            builder.AppendLine();
        }

        builder.AppendLine(".e");
        return builder.ToString();
    }

    private static string WritePla(PlaData pla, int outputType)
    {
        using var writer = new StringWriter();
        PlaWriter.Write(writer, pla, outputType);
        return writer.ToString();
    }

    private static List<MinimizedProductTerm> ParseMinimizedProducts(
        string plaText,
        int outputCount)
    {
        var products = new List<MinimizedProductTerm>();
        foreach (var line in plaText.Split(
            ['\r', '\n'],
            StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (line.Length == 0 || line[0] == '.')
            {
                continue;
            }

            var parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length < 2)
            {
                continue;
            }

            var outputPattern = parts[^1];
            if (outputPattern.Length != outputCount)
            {
                continue;
            }

            products.Add(new MinimizedProductTerm(
                parts[0],
                outputPattern.Select(static value => value == '1' ? "1" : "0").ToArray()));
        }

        return products;
    }

    private static string GenerateMinimizedEquation(
        string[] inputNames,
        string[] outputNames,
        IReadOnlyList<MinimizedProductTerm> products)
    {
        var equations = new List<string>
        {
            "Minimized:"
        };

        for (var outputIndex = 0; outputIndex < outputNames.Length; outputIndex++)
        {
            var outputProducts = products
                .Where(product => product.OutputValues[outputIndex] == "1")
                .Select(product => BuildProductTerm(product.InputPattern, inputNames))
                .ToArray();

            var expression = outputProducts.Length == 0
                ? "0"
                : string.Join(" + ", outputProducts);
            equations.Add($"{outputNames[outputIndex]} = {expression};");
        }

        return string.Join(Environment.NewLine, equations);
    }

    private static string BuildMinimizedPla(
        string[] inputNames,
        string[] outputNames,
        IReadOnlyList<MinimizedProductTerm> products)
    {
        var builder = new StringBuilder();
        builder.AppendLine($".i {inputNames.Length}");
        builder.AppendLine($".o {outputNames.Length}");
        builder.Append(".ilb");
        foreach (var inputName in inputNames)
        {
            builder.Append(' ').Append(inputName);
        }

        builder.AppendLine();
        builder.Append(".ob");
        foreach (var outputName in outputNames)
        {
            builder.Append(' ').Append(outputName);
        }

        builder.AppendLine();
        builder.AppendLine(".type f");
        builder.AppendLine($".p {products.Count}");
        foreach (var product in products)
        {
            builder
                .Append(product.InputPattern)
                .Append(' ')
                .AppendJoin("", product.OutputValues)
                .AppendLine();
        }

        builder.AppendLine(".e");
        return builder.ToString();
    }

    private static string BuildProductTerm(string inputPattern, string[] inputNames)
    {
        var literals = new List<string>();
        for (var inputIndex = 0; inputIndex < inputNames.Length; inputIndex++)
        {
            literals.Add(inputPattern[inputIndex] switch
            {
                '0' => $"{inputNames[inputIndex]}'",
                '1' => inputNames[inputIndex],
                _ => ""
            });
        }

        literals.RemoveAll(static literal => literal.Length == 0);
        return literals.Count == 0 ? "1" : string.Join(" ", literals);
    }

    private static string FormatInputPattern(int term, int inputCount)
    {
        var chars = new char[inputCount];
        for (var inputIndex = 0; inputIndex < inputCount; inputIndex++)
        {
            var bitOffset = inputCount - inputIndex - 1;
            chars[inputIndex] = ((term >> bitOffset) & 1) == 0 ? '0' : '1';
        }

        return new string(chars);
    }
}

public sealed record MinimizeOptions(
    bool UseExactMode,
    bool MinimizeOutputsIndependently)
{
    public static MinimizeOptions Default { get; } = new(
        UseExactMode: false,
        MinimizeOutputsIndependently: true);
}
