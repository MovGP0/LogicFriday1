using System.Text;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class TruthTableCsvExportService
{
    public static string Export(
        LogicFunction logicFunction,
        bool useMinimizedFunction = false)
    {
        var builder = new StringBuilder();
        AppendRow(
            builder,
            logicFunction.InputNames
                .Concat([""])
                .Concat(logicFunction.OutputNames));

        if (useMinimizedFunction && logicFunction.MinimizedFunction is { } minimizedFunction)
        {
            foreach (var product in minimizedFunction.Products)
            {
                AppendRow(
                    builder,
                    product.InputPattern
                        .Select(static value => value == '-' ? "X" : value.ToString())
                        .Concat([""])
                        .Concat(product.OutputValues.Select(FormatTruthTableValue)));
            }

            return builder.ToString();
        }

        for (var term = 0; term < logicFunction.OutputValues.Count; term++)
        {
            AppendRow(
                builder,
                FormatInputValues(term, logicFunction.InputNames.Length)
                    .Concat([""])
                    .Concat(logicFunction.OutputValues[term].Select(FormatTruthTableValue)));
        }

        return builder.ToString();
    }

    private static IEnumerable<string> FormatInputValues(int term, int inputCount)
    {
        for (var inputIndex = 0; inputIndex < inputCount; inputIndex++)
        {
            var bitOffset = inputCount - inputIndex - 1;
            yield return (((term >> bitOffset) & 1) == 0 ? 0 : 1).ToString();
        }
    }

    private static string FormatTruthTableValue(string value)
    {
        return value switch
        {
            "0" or "1" or "X" => value,
            _ => throw new InvalidOperationException($"Invalid truth-table value '{value}'.")
        };
    }

    private static void AppendRow(StringBuilder builder, IEnumerable<string> values)
    {
        var isFirst = true;
        foreach (var value in values)
        {
            if (!isFirst)
            {
                builder.Append(',');
            }

            builder.Append(Escape(value));
            isFirst = false;
        }

        builder.Append("\r\n");
    }

    private static string Escape(string value)
    {
        if (!value.Contains(',') &&
            !value.Contains('"') &&
            !value.Contains('\r') &&
            !value.Contains('\n'))
        {
            return value;
        }

        return $"\"{value.Replace("\"", "\"\"")}\"";
    }
}
