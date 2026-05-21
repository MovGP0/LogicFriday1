using System.Globalization;

namespace LogicFriday1.Services;

public static class TruthTableImporter
{
    private const int MinimumInputCount = 2;
    private const int MaximumInputCount = 16;
    private const int MinimumOutputCount = 1;
    private const int MaximumOutputCount = 16;
    private const int MaximumVariableNameLength = 8;

    public static TruthTableImportResult Import(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        var lines = ReadMeaningfulLines(text).ToArray();
        if (lines.Length == 0)
        {
            throw new TruthTableImportException("The selected file does not contain a truth table.");
        }

        var header = ParseHeader(lines[0].Text, lines[0].LineNumber);
        var rowCount = 1 << header.InputNames.Length;
        var outputValues = CreateOutputValues(rowCount, header.OutputNames.Length);
        var assigned = new bool[rowCount];

        for (var lineIndex = 1; lineIndex < lines.Length; lineIndex++)
        {
            var rowParts = SplitFields(lines[lineIndex].Text);
            if (header.HasTermColumn && rowParts.Length > 0)
            {
                rowParts = rowParts[1..];
            }

            ApplyRow(
                rowParts,
                lines[lineIndex].LineNumber,
                header.InputNames.Length,
                header.OutputNames.Length,
                outputValues,
                assigned);
        }

        return new TruthTableImportResult(header.InputNames, header.OutputNames, outputValues);
    }

    private static IEnumerable<TruthTableImportLine> ReadMeaningfulLines(string text)
    {
        using var reader = new StringReader(text);
        var lineNumber = 0;
        while (reader.ReadLine() is { } line)
        {
            lineNumber++;

            if (line.Length == 0 || line[0] == '%' || string.IsNullOrWhiteSpace(line))
            {
                continue;
            }

            yield return new TruthTableImportLine(line, lineNumber);
        }
    }

    private static TruthTableHeader ParseHeader(string line, int lineNumber)
    {
        var parts = SplitFields(line);
        var arrowIndex = Array.FindIndex(parts, static part => part == "=>");
        if (parts.Length > 0 &&
            parts[0].Equals("Term", StringComparison.OrdinalIgnoreCase) &&
            arrowIndex > 1 &&
            arrowIndex < parts.Length - 1)
        {
            var gridInputNames = parts[1..arrowIndex];
            var gridOutputNames = parts[(arrowIndex + 1)..].Where(static x => x.Length > 0).ToArray();

            ValidateVariableNames(gridInputNames, gridOutputNames, lineNumber);
            return new TruthTableHeader(gridInputNames, gridOutputNames, true);
        }

        var separatorIndex = Array.FindIndex(parts, string.IsNullOrEmpty);
        if (separatorIndex <= 0 || separatorIndex >= parts.Length - 1)
        {
            throw new TruthTableImportException(
                $"Line {lineNumber} must contain input names, an empty separator field, and output names.");
        }

        var inputNames = parts[..separatorIndex];
        var outputNames = parts[(separatorIndex + 1)..].Where(static x => x.Length > 0).ToArray();

        ValidateVariableNames(inputNames, outputNames, lineNumber);
        return new TruthTableHeader(inputNames, outputNames, false);
    }

    private static void ValidateVariableNames(string[] inputNames, string[] outputNames, int lineNumber)
    {
        if (inputNames.Length is < MinimumInputCount or > MaximumInputCount)
        {
            throw new TruthTableImportException(
                $"Line {lineNumber} must define between {MinimumInputCount} and {MaximumInputCount} inputs.");
        }

        if (outputNames.Length is < MinimumOutputCount or > MaximumOutputCount)
        {
            throw new TruthTableImportException(
                $"Line {lineNumber} must define between {MinimumOutputCount} and {MaximumOutputCount} outputs.");
        }

        foreach (var name in inputNames.Concat(outputNames))
        {
            if (name.Length > MaximumVariableNameLength)
            {
                throw new TruthTableImportException(
                    $"Line {lineNumber} contains a variable name longer than {MaximumVariableNameLength} characters.");
            }

            if (IsTruthValue(name[0]))
            {
                throw new TruthTableImportException(
                    $"Line {lineNumber} contains a variable name that looks like a truth-table value.");
            }
        }

        var duplicateName = inputNames
            .Concat(outputNames)
            .GroupBy(static name => name, StringComparer.OrdinalIgnoreCase)
            .FirstOrDefault(static group => group.Count() > 1)
            ?.Key;

        if (duplicateName is not null)
        {
            throw new TruthTableImportException(
                $"Line {lineNumber} contains duplicate variable name '{duplicateName}'.");
        }
    }

    private static string[][] CreateOutputValues(int rowCount, int outputCount)
    {
        var values = new string[rowCount][];
        for (var rowIndex = 0; rowIndex < rowCount; rowIndex++)
        {
            values[rowIndex] = Enumerable.Repeat("0", outputCount).ToArray();
        }

        return values;
    }

    private static void ApplyRow(
        string[] parts,
        int lineNumber,
        int inputCount,
        int outputCount,
        string[][] outputValues,
        bool[] assigned)
    {
        var (inputPattern, outputPattern) = ParseRowPatterns(parts, lineNumber);
        if (inputPattern.Length != inputCount || outputPattern.Length != outputCount)
        {
            throw new TruthTableImportException(
                $"Line {lineNumber} does not match the declared input/output count.");
        }

        ValidatePattern(inputPattern, lineNumber);
        ValidatePattern(outputPattern, lineNumber);

        foreach (var term in ExpandInputPattern(inputPattern))
        {
            if (assigned[term])
            {
                for (var outputIndex = 0; outputIndex < outputCount; outputIndex++)
                {
                    if (outputValues[term][outputIndex] != outputPattern[outputIndex].ToString())
                    {
                        throw new TruthTableImportException(
                            $"Line {lineNumber} conflicts with an output value assigned earlier.");
                    }
                }

                continue;
            }

            assigned[term] = true;
            for (var outputIndex = 0; outputIndex < outputCount; outputIndex++)
            {
                outputValues[term][outputIndex] = outputPattern[outputIndex].ToString();
            }
        }
    }

    private static (string InputPattern, string OutputPattern) ParseRowPatterns(string[] parts, int lineNumber)
    {
        var separatorIndex = Array.FindIndex(parts, string.IsNullOrEmpty);
        if (separatorIndex >= 0)
        {
            return NormalizePatterns(
                string.Concat(parts[..separatorIndex]),
                string.Concat(parts[(separatorIndex + 1)..]));
        }

        var populatedParts = parts.Where(static part => part.Length > 0).ToArray();
        if (populatedParts.Length == 2)
        {
            return NormalizePatterns(populatedParts[0], populatedParts[1]);
        }

        throw new TruthTableImportException(
            $"Line {lineNumber} must contain input values, an empty separator field, and output values.");
    }

    private static (string InputPattern, string OutputPattern) NormalizePatterns(
        string inputPattern,
        string outputPattern)
    {
        return (
            inputPattern.ToUpper(CultureInfo.InvariantCulture),
            outputPattern.ToUpper(CultureInfo.InvariantCulture));
    }

    private static void ValidatePattern(string pattern, int lineNumber)
    {
        foreach (var value in pattern)
        {
            if (!IsTruthValue(value))
            {
                throw new TruthTableImportException(
                    $"Line {lineNumber} contains invalid truth-table value '{value}'.");
            }
        }
    }

    private static IEnumerable<int> ExpandInputPattern(string inputPattern)
    {
        var terms = new List<int>
        {
            0
        };

        for (var inputIndex = 0; inputIndex < inputPattern.Length; inputIndex++)
        {
            var bitOffset = inputPattern.Length - inputIndex - 1;
            var bit = 1 << bitOffset;
            switch (inputPattern[inputIndex])
            {
                case '0':
                    break;
                case '1':
                    for (var termIndex = 0; termIndex < terms.Count; termIndex++)
                    {
                        terms[termIndex] |= bit;
                    }

                    break;
                case 'X':
                    var currentCount = terms.Count;
                    for (var termIndex = 0; termIndex < currentCount; termIndex++)
                    {
                        terms.Add(terms[termIndex] | bit);
                    }

                    break;
            }
        }

        return terms;
    }

    private static string[] SplitFields(string line)
    {
        return line
            .Split([ ',', '\t' ], StringSplitOptions.None)
            .Select(static part => part.Trim().Trim('"'))
            .ToArray();
    }

    private static bool IsTruthValue(char value)
    {
        return value is '0' or '1' or 'X' or 'x';
    }

    private sealed record TruthTableHeader(string[] InputNames, string[] OutputNames, bool HasTermColumn);

    private sealed record TruthTableImportLine(string Text, int LineNumber);
}
