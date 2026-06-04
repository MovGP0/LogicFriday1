using System.Globalization;
using System.Text;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class LookupFunctionGenerator
{
    public static LookupFunctionGenerationResult Generate(
        LogicFunction logicFunction,
        LookupFunctionLanguage language)
    {
        ArgumentNullException.ThrowIfNull(logicFunction);

        var definition = GetLanguageDefinition(language);
        var context = LookupFunctionContext.Create(logicFunction, definition.IdentifierStyle);
        var source = language switch
        {
            LookupFunctionLanguage.Vhdl => GenerateVhdl(context),
            LookupFunctionLanguage.Verilog => GenerateVerilog(context),
            LookupFunctionLanguage.C => GenerateC(context),
            LookupFunctionLanguage.Rust => GenerateRust(context),
            LookupFunctionLanguage.CSharp => GenerateCSharp(context),
            LookupFunctionLanguage.Java => GenerateJava(context),
            LookupFunctionLanguage.Python => GeneratePython(context),
            LookupFunctionLanguage.JavaScript => GenerateJavaScript(context),
            LookupFunctionLanguage.Go => GenerateGo(context),
            _ => throw new ArgumentOutOfRangeException(nameof(language), language, "Unsupported lookup function language.")
        };

        return new LookupFunctionGenerationResult(
            $"lookup{definition.FileExtension}",
            definition.FileExtension,
            definition.FileTypeName,
            source);
    }

    public static string GetDisplayName(LookupFunctionLanguage language) => GetLanguageDefinition(language).DisplayName;

    public static string GetFileExtension(LookupFunctionLanguage language) => GetLanguageDefinition(language).FileExtension;

    public static string GetFileTypeName(LookupFunctionLanguage language) => GetLanguageDefinition(language).FileTypeName;

    private static string GenerateC(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "C", "//", context);
        builder.AppendLine("#include <stddef.h>");
        builder.AppendLine();
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"static const unsigned char {output.TableName}[] = {{ {JoinValues(output.Values)} }};");
            builder.AppendLine();
            builder.AppendLine($"int {output.FunctionName}(unsigned int nTerm)");
            builder.AppendLine("{");
            builder.AppendLine($"    if (nTerm >= {context.RowCount}U)");
            builder.AppendLine("    {");
            builder.AppendLine("        return 0;");
            builder.AppendLine("    }");
            builder.AppendLine();
            builder.AppendLine($"    return {output.TableName}[nTerm] != 0U ? 1 : 0;");
            builder.AppendLine("}");
            builder.AppendLine();
        }

        return builder.ToString();
    }

    private static string GenerateRust(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "Rust", "//", context);
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"const {output.TableName}: [u8; {context.RowCount}] = [{JoinValues(output.Values)}];");
            builder.AppendLine();
            builder.AppendLine($"pub fn {output.FunctionName}(n_term: usize) -> bool {{");
            builder.AppendLine($"    {output.TableName}.get(n_term).copied().unwrap_or(0) != 0");
            builder.AppendLine("}");
            builder.AppendLine();
        }

        return builder.ToString();
    }

    private static string GenerateCSharp(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "C#", "//", context);
        builder.AppendLine("public static class LookupFunctions");
        builder.AppendLine("{");
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"    private static readonly byte[] {output.TableName} = [{JoinValues(output.Values)}];");
            builder.AppendLine();
            builder.AppendLine($"    public static bool {output.FunctionName}(int nTerm)");
            builder.AppendLine("    {");
            builder.AppendLine($"        return nTerm >= 0 && nTerm < {output.TableName}.Length && {output.TableName}[nTerm] != 0;");
            builder.AppendLine("    }");
            builder.AppendLine();
        }

        TrimTrailingBlankLine(builder);
        builder.AppendLine("}");
        return builder.ToString();
    }

    private static string GenerateJava(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "Java", "//", context);
        builder.AppendLine("final class LookupFunctions {");
        builder.AppendLine("    private LookupFunctions() { }");
        builder.AppendLine();
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"    private static final byte[] {output.TableName} = new byte[] {{ {JoinValues(output.Values)} }};");
            builder.AppendLine();
            builder.AppendLine($"    public static boolean {output.FunctionName}(int nTerm) {{");
            builder.AppendLine($"        return nTerm >= 0 && nTerm < {output.TableName}.length && {output.TableName}[nTerm] != 0;");
            builder.AppendLine("    }");
            builder.AppendLine();
        }

        TrimTrailingBlankLine(builder);
        builder.AppendLine("}");
        return builder.ToString();
    }

    private static string GeneratePython(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "Python", "#", context);
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"{output.TableName} = [{JoinValues(output.Values)}]");
            builder.AppendLine();
            builder.AppendLine($"def {output.FunctionName}(n_term: int) -> bool:");
            builder.AppendLine($"    return 0 <= n_term < len({output.TableName}) and {output.TableName}[n_term] != 0");
            builder.AppendLine();
        }

        return builder.ToString();
    }

    private static string GenerateJavaScript(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "JavaScript", "//", context);
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"const {output.TableName} = [{JoinValues(output.Values)}];");
            builder.AppendLine();
            builder.AppendLine($"export function {output.FunctionName}(nTerm) {{");
            builder.AppendLine($"    return Number.isInteger(nTerm) && nTerm >= 0 && nTerm < {output.TableName}.length && {output.TableName}[nTerm] !== 0;");
            builder.AppendLine("}");
            builder.AppendLine();
        }

        return builder.ToString();
    }

    private static string GenerateGo(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "Go", "//", context);
        builder.AppendLine("package lookup");
        builder.AppendLine();
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"var {output.TableName} = []byte{{ {JoinValues(output.Values)} }}");
            builder.AppendLine();
            builder.AppendLine($"func {output.FunctionName}(nTerm int) bool {{");
            builder.AppendLine($"    return nTerm >= 0 && nTerm < len({output.TableName}) && {output.TableName}[nTerm] != 0");
            builder.AppendLine("}");
            builder.AppendLine();
        }

        return builder.ToString();
    }

    private static string GenerateVhdl(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "VHDL", "--", context);
        builder.AppendLine("library ieee;");
        builder.AppendLine("use ieee.std_logic_1164.all;");
        builder.AppendLine();
        builder.AppendLine("entity lookup_function is");
        builder.AppendLine("    port (");
        builder.AppendLine($"        n_term : in natural range 0 to {context.RowCount - 1};");
        for (var outputIndex = 0; outputIndex < context.Outputs.Count; outputIndex++)
        {
            var suffix = outputIndex == context.Outputs.Count - 1 ? "" : ";";
            builder.AppendLine($"        {context.Outputs[outputIndex].SignalName} : out std_logic{suffix}");
        }

        builder.AppendLine("    );");
        builder.AppendLine("end entity lookup_function;");
        builder.AppendLine();
        builder.AppendLine("architecture rtl of lookup_function is");
        builder.AppendLine($"    type lookup_table is array (0 to {context.RowCount - 1}) of std_logic;");
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"    constant {output.TableName} : lookup_table := ({JoinQuotedBits(output.Values, "'")});");
        }

        builder.AppendLine("begin");
        foreach (var output in context.Outputs)
        {
            builder.AppendLine($"    {output.SignalName} <= {output.TableName}(n_term);");
        }

        builder.AppendLine("end architecture rtl;");
        return builder.ToString();
    }

    private static string GenerateVerilog(LookupFunctionContext context)
    {
        var builder = new StringBuilder();
        AppendHeader(builder, "Verilog", "//", context);
        var termWidth = Math.Max(1, (int)Math.Ceiling(Math.Log2(context.RowCount)));
        builder.AppendLine("module lookup_function(");
        builder.AppendLine($"    input wire [{termWidth - 1}:0] n_term,");
        for (var outputIndex = 0; outputIndex < context.Outputs.Count; outputIndex++)
        {
            var suffix = outputIndex == context.Outputs.Count - 1 ? "" : ",";
            builder.AppendLine($"    output wire {context.Outputs[outputIndex].SignalName}{suffix}");
        }

        builder.AppendLine(");");
        builder.AppendLine($"    reg [{context.Outputs.Count - 1}:0] lookup_value;");
        builder.AppendLine();
        builder.AppendLine("    always @* begin");
        builder.AppendLine("        case (n_term)");
        for (var term = 0; term < context.RowCount; term++)
        {
            var bits = string.Concat(context.Outputs.Select(output => output.Values[term]).Reverse());
            builder.AppendLine($"            {termWidth}'d{term}: lookup_value = {context.Outputs.Count}'b{bits};");
        }

        builder.AppendLine($"            default: lookup_value = {context.Outputs.Count}'b{new string('0', context.Outputs.Count)};");
        builder.AppendLine("        endcase");
        builder.AppendLine("    end");
        builder.AppendLine();
        for (var outputIndex = 0; outputIndex < context.Outputs.Count; outputIndex++)
        {
            builder.AppendLine($"    assign {context.Outputs[outputIndex].SignalName} = lookup_value[{outputIndex}];");
        }

        builder.AppendLine("endmodule");
        return builder.ToString();
    }

    private static void AppendHeader(
        StringBuilder builder,
        string languageName,
        string commentPrefix,
        LookupFunctionContext context)
    {
        builder.AppendLine($"{commentPrefix} {languageName} lookup functions generated by LogicFriday1");
        builder.AppendLine($"{commentPrefix} Inputs: {string.Join(", ", context.InputNames)}");
        builder.AppendLine($"{commentPrefix} Outputs: {string.Join(", ", context.Outputs.Select(static output => output.OriginalName))}");
        builder.AppendLine($"{commentPrefix} Rows with don't-care output values are emitted as 0.");
        builder.AppendLine();
    }

    private static LookupFunctionLanguageDefinition GetLanguageDefinition(LookupFunctionLanguage language)
    {
        return language switch
        {
            LookupFunctionLanguage.Vhdl => new("VHDL", ".vhd", "VHDL Files", IdentifierStyle.CaseInsensitive),
            LookupFunctionLanguage.Verilog => new("Verilog", ".v", "Verilog Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.C => new("C", ".c", "C Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.Rust => new("Rust", ".rs", "Rust Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.CSharp => new("C#", ".cs", "C# Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.Java => new("Java", ".java", "Java Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.Python => new("Python", ".py", "Python Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.JavaScript => new("JavaScript", ".js", "JavaScript Source Files", IdentifierStyle.CaseSensitive),
            LookupFunctionLanguage.Go => new("Go", ".go", "Go Source Files", IdentifierStyle.CaseSensitive),
            _ => throw new ArgumentOutOfRangeException(nameof(language), language, "Unsupported lookup function language.")
        };
    }

    private static string JoinValues(IReadOnlyList<string> values) => string.Join(", ", values);

    private static string JoinQuotedBits(IReadOnlyList<string> values, string quote)
    {
        return string.Join(", ", values.Select(value => $"{quote}{value}{quote}"));
    }

    private static void TrimTrailingBlankLine(StringBuilder builder)
    {
        while (builder.Length >= Environment.NewLine.Length * 2 &&
            builder.ToString(builder.Length - (Environment.NewLine.Length * 2), Environment.NewLine.Length * 2) ==
            Environment.NewLine + Environment.NewLine)
        {
            builder.Length -= Environment.NewLine.Length;
        }
    }

    private sealed record LookupFunctionLanguageDefinition(
        string DisplayName,
        string FileExtension,
        string FileTypeName,
        IdentifierStyle IdentifierStyle);

    private sealed record LookupFunctionContext(
        string[] InputNames,
        int RowCount,
        IReadOnlyList<LookupFunctionOutput> Outputs)
    {
        public static LookupFunctionContext Create(LogicFunction logicFunction, IdentifierStyle identifierStyle)
        {
            var rowCount = logicFunction.OutputValues.Count;
            var usedNames = new HashSet<string>(
                identifierStyle == IdentifierStyle.CaseInsensitive
                    ? StringComparer.OrdinalIgnoreCase
                    : StringComparer.Ordinal);
            var outputs = logicFunction.OutputNames
                .Select((name, outputIndex) =>
                {
                    var identifier = CreateUniqueIdentifier(name, usedNames);
                    var values = logicFunction.OutputValues
                        .Select(row => row[outputIndex] == "1" ? "1" : "0")
                        .ToArray();

                    return new LookupFunctionOutput(
                        name,
                        identifier,
                        $"lu_{identifier}",
                        $"LU_{identifier.ToUpper(CultureInfo.InvariantCulture)}",
                        values);
                })
                .ToArray();

            return new LookupFunctionContext(logicFunction.InputNames, rowCount, outputs);
        }
    }

    private sealed record LookupFunctionOutput(
        string OriginalName,
        string SignalName,
        string FunctionName,
        string TableName,
        string[] Values);

    private static string CreateUniqueIdentifier(string name, HashSet<string> usedNames)
    {
        var identifier = SanitizeIdentifier(name);
        var uniqueIdentifier = identifier;
        var suffix = 2;
        while (!usedNames.Add(uniqueIdentifier))
        {
            uniqueIdentifier = $"{identifier}_{suffix}";
            suffix++;
        }

        return uniqueIdentifier;
    }

    private static string SanitizeIdentifier(string name)
    {
        var builder = new StringBuilder();
        foreach (var character in name)
        {
            builder.Append(char.IsLetterOrDigit(character) || character == '_' ? character : '_');
        }

        if (builder.Length == 0)
        {
            builder.Append("output");
        }

        if (char.IsDigit(builder[0]))
        {
            builder.Insert(0, '_');
        }

        return builder.ToString();
    }

    private enum IdentifierStyle
    {
        CaseSensitive,
        CaseInsensitive
    }
}
