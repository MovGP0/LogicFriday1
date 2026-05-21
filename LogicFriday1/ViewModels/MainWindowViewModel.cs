using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Models;

namespace LogicFriday1.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    private string[] _truthTableInputNames = [];
    private string[] _truthTableOutputNames = [];

    [ObservableProperty]
    private string _statusText = "Ready";

    [ObservableProperty]
    private string _logicEquationText = "";

    [ObservableProperty]
    private bool _isEquationEditorVisible;

    [ObservableProperty]
    private bool _isTruthTableVisible;

    [ObservableProperty]
    private bool _isGateDiagramVisible;

    [ObservableProperty]
    private bool _isFunctionDetailVisible;

    [ObservableProperty]
    private FunctionSummaryRow? _selectedFunctionSummary;

    [ObservableProperty]
    private GatePaletteItem? _selectedGatePaletteItem;

    public ObservableCollection<TruthTableRow> TruthTableRows { get; } = [];

    public ObservableCollection<TruthTableRow> FunctionTruthTableRows { get; } = [];

    public ObservableCollection<GatePaletteItem> GatePaletteItems { get; } =
    [
        new("Select", GatePaletteKind.Select, 0x42b, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("Wire", GatePaletteKind.Wire, 0x42c, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("Inverter", GatePaletteKind.Not, 0x3f4, 1, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In NAND", GatePaletteKind.Nand, 0x3f5, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("3-In NAND", GatePaletteKind.Nand, 0x3f6, 3, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("4-In NAND", GatePaletteKind.Nand, 0x3f7, 4, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In NOR", GatePaletteKind.Nor, 0x3f8, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("3-In NOR", GatePaletteKind.Nor, 0x3f9, 3, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("4-In NOR", GatePaletteKind.Nor, 0x3fa, 4, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In MUX", GatePaletteKind.Mux, 0x3fc, 3, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In AND", GatePaletteKind.And, 0x3fd, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("3-In AND", GatePaletteKind.And, 0x3fe, 3, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("4-In AND", GatePaletteKind.And, 0x3ff, 4, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In OR", GatePaletteKind.Or, 0x400, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("3-In OR", GatePaletteKind.Or, 0x401, 3, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("4-In OR", GatePaletteKind.Or, 0x402, 4, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("CONST 0", GatePaletteKind.ConstantZero, 0x42a, 0, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("CONST 1", GatePaletteKind.ConstantOne, 0x408, 0, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("2-In XOR", GatePaletteKind.Xor, 0x430, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("Input", GatePaletteKind.Input, 0x438, 0, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("Output", GatePaletteKind.Output, 0x439, 1, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("Submit", GatePaletteKind.Submit, 0x458, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("Cancel", GatePaletteKind.Cancel, 0x45a, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("Help", GatePaletteKind.Help, 0x428, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c")
    ];

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsUnminimizedViewSelected))]
    private bool _isMinimizedViewSelected;

    public ObservableCollection<FunctionSummaryRow> FunctionSummaries { get; } =
    [
        new()
        {
            Function = "<none>"
        }
    ];

    public bool IsUnminimizedViewSelected
    {
        get => !IsMinimizedViewSelected;
        set => IsMinimizedViewSelected = !value;
    }

    public void ShowMinimizedView()
    {
        IsMinimizedViewSelected = true;
        StatusText = "Showing minimized function view";
    }

    public void StartNewLogicEquation()
    {
        LogicEquationText = "";
        IsEquationEditorVisible = true;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Entering new logic equation";
    }

    public void StartNewTruthTable(string[] inputNames, string[] outputNames)
    {
        var outputValues = Enumerable
            .Range(0, 1 << inputNames.Length)
            .Select(_ => Enumerable.Repeat("0", outputNames.Length).ToArray())
            .ToArray();

        StartTruthTable(inputNames, outputNames, outputValues, "Editing truth table");
    }

    public void StartImportedTruthTable(string[] inputNames, string[] outputNames, IReadOnlyList<string[]> outputValues)
    {
        StartTruthTable(inputNames, outputNames, outputValues, "Imported truth table");
    }

    private void StartTruthTable(
        string[] inputNames,
        string[] outputNames,
        IReadOnlyList<string[]> outputValues,
        string statusPrefix)
    {
        TruthTableRows.Clear();
        _truthTableInputNames = [.. inputNames];
        _truthTableOutputNames = [.. outputNames];

        var rowCount = 1 << inputNames.Length;
        for (var term = 0; term < rowCount; term++)
        {
            var cells = new List<TruthTableCell>
            {
                new(term.ToString(), false)
            };

            for (var inputIndex = 0; inputIndex < inputNames.Length; inputIndex++)
            {
                var bitOffset = inputNames.Length - inputIndex - 1;
                var value = ((term >> bitOffset) & 1).ToString();
                cells.Add(new TruthTableCell(value, false));
            }

            cells.Add(new TruthTableCell("", false));

            for (var outputIndex = 0; outputIndex < outputNames.Length; outputIndex++)
            {
                cells.Add(new TruthTableCell(outputValues[term][outputIndex], true));
            }

            TruthTableRows.Add(new TruthTableRow(cells));
        }

        IsEquationEditorVisible = false;
        IsTruthTableVisible = true;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = $"{statusPrefix}: {inputNames.Length} inputs, {outputNames.Length} outputs";
    }

    public void StartNewGateDiagram()
    {
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = true;
        IsFunctionDetailVisible = false;
        SelectedGatePaletteItem = null;
        StatusText = "Editing gate diagram";
    }

    public void SubmitLogicEquationEditing()
    {
        if (!IsEquationEditorVisible)
        {
            StatusText = "No logic equation is active";
            return;
        }

        StatusText = "Logic equation submit is not implemented";
    }

    public void CancelLogicEquationEditing()
    {
        LogicEquationText = "";
        IsEquationEditorVisible = false;
        StatusText = "Ready";
    }

    public void CancelTruthTableEditing()
    {
        TruthTableRows.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        IsTruthTableVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Ready";
    }

    public void SubmitTruthTableEditing()
    {
        if (_truthTableInputNames.Length == 0 || _truthTableOutputNames.Length == 0)
        {
            StatusText = "No truth table is active";
            return;
        }

        var logicFunction = CreateTruthTableFunction();
        AddFunction(logicFunction);
        ShowFunction(logicFunction);
        TruthTableRows.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        StatusText = "Truth table submitted";
    }

    public void CloseCurrentDocument()
    {
        LogicEquationText = "";
        TruthTableRows.Clear();
        FunctionTruthTableRows.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        SelectedGatePaletteItem = null;
        SelectedFunctionSummary = null;
        FunctionSummaries.Clear();
        FunctionSummaries.Add(new FunctionSummaryRow
        {
            Function = "<none>"
        });
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Ready";
    }

    public void SelectGatePaletteItem(GatePaletteItem item)
    {
        SelectedGatePaletteItem = item;
        StatusText = $"Selected gate diagram tool: {item.Label}";
    }

    public LogicFunction? GetSelectedFunction()
    {
        return SelectedFunctionSummary?.LogicFunction;
    }

    public void ShowFunction(LogicFunction logicFunction)
    {
        LogicEquationText = logicFunction.EquationText;
        RefreshFunctionTruthTable(logicFunction);
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = true;
        StatusText = $"Showing {logicFunction.OutputNames.Length} output function";
    }

    private LogicFunction CreateTruthTableFunction()
    {
        var outputStartIndex = _truthTableInputNames.Length + 2;
        var outputValues = TruthTableRows
            .Select(row => Enumerable
                .Range(0, _truthTableOutputNames.Length)
                .Select(outputIndex => row.Cells[outputStartIndex + outputIndex].Value)
                .ToArray())
            .ToArray();

        return new TruthTableLogicFunction(
            [.. _truthTableInputNames],
            [.. _truthTableOutputNames],
            outputValues,
            GenerateSumOfProductsEquation(
                _truthTableInputNames,
                _truthTableOutputNames,
                outputValues,
                "Entered by truthtable:"));
    }

    private string GenerateSumOfProductsEquation(
        string[] inputNames,
        string[] outputNames,
        IReadOnlyList<string[]> outputValues,
        string label)
    {
        var equations = new List<string>
        {
            label
        };

        for (var outputIndex = 0; outputIndex < outputNames.Length; outputIndex++)
        {
            var trueTerms = outputValues
                .Select((outputs, term) => new
                {
                    outputs,
                    term
                })
                .Where(row => row.outputs[outputIndex] == "1")
                .ToArray();

            if (trueTerms.Length == outputValues.Count)
            {
                equations.Add($"{outputNames[outputIndex]} = 1;");
            }
            else if (trueTerms.Length == 0)
            {
                equations.Add($"{outputNames[outputIndex]} = 0;");
            }
            else
            {
                var terms = trueTerms.Select(row => BuildProductTerm(row.term, inputNames));
                equations.Add($"{outputNames[outputIndex]} = {string.Join(" + ", terms)};");
            }
        }

        return string.Join(Environment.NewLine, equations);
    }

    private static string BuildProductTerm(int term, string[] inputNames)
    {
        var literals = new List<string>();
        for (var inputIndex = 0; inputIndex < inputNames.Length; inputIndex++)
        {
            var bitOffset = inputNames.Length - inputIndex - 1;
            var inputValue = (term >> bitOffset) & 1;
            literals.Add(inputValue == 0
                ? $"{inputNames[inputIndex]}'"
                : inputNames[inputIndex]);
        }

        return string.Join(" ", literals);
    }

    private void AddFunction(LogicFunction logicFunction)
    {
        if (FunctionSummaries.Count == 1 && FunctionSummaries[0].LogicFunction is null)
        {
            FunctionSummaries.Clear();
        }

        var summary = CreateFunctionSummary(logicFunction);
        FunctionSummaries.Add(summary);
        SelectedFunctionSummary = summary;
    }

    private static FunctionSummaryRow CreateFunctionSummary(LogicFunction logicFunction)
    {
        var trueCounts = new int[logicFunction.OutputNames.Length];
        var falseCounts = new int[logicFunction.OutputNames.Length];
        var dontCareCounts = new int[logicFunction.OutputNames.Length];

        foreach (var row in logicFunction.OutputValues)
        {
            for (var outputIndex = 0; outputIndex < logicFunction.OutputNames.Length; outputIndex++)
            {
                switch (row[outputIndex])
                {
                    case "1":
                        trueCounts[outputIndex]++;
                        break;
                    case "X":
                        dontCareCounts[outputIndex]++;
                        break;
                    default:
                        falseCounts[outputIndex]++;
                        break;
                }
            }
        }

        return new FunctionSummaryRow(
            Function: FormatFunctionName(logicFunction.OutputNames),
            Inputs: logicFunction.InputNames.Length.ToString(),
            Outputs: logicFunction.OutputNames.Length.ToString(),
            True: string.Join(", ", trueCounts),
            False: string.Join(", ", falseCounts),
            DC: string.Join(", ", dontCareCounts),
            PI: "Unminimized",
            Gates: "Not Mapped",
            LogicFunction: logicFunction);
    }

    private void RefreshFunctionTruthTable(LogicFunction logicFunction)
    {
        FunctionTruthTableRows.Clear();

        for (var term = 0; term < logicFunction.OutputValues.Count; term++)
        {
            var outputs = logicFunction.OutputValues[term];
            if (outputs.All(static value => value == "0"))
            {
                continue;
            }

            FunctionTruthTableRows.Add(CreateTruthTableRow(
                term,
                logicFunction.InputNames,
                outputs,
                outputCellsEditable: false));
        }
    }

    private static TruthTableRow CreateTruthTableRow(
        int term,
        string[] inputNames,
        IReadOnlyList<string> outputValues,
        bool outputCellsEditable)
    {
        var cells = new List<TruthTableCell>
        {
            new(term.ToString(), false)
        };

        for (var inputIndex = 0; inputIndex < inputNames.Length; inputIndex++)
        {
            var bitOffset = inputNames.Length - inputIndex - 1;
            var value = ((term >> bitOffset) & 1).ToString();
            cells.Add(new TruthTableCell(value, false));
        }

        cells.Add(new TruthTableCell("", false));

        foreach (var outputValue in outputValues)
        {
            cells.Add(new TruthTableCell(outputValue, outputCellsEditable));
        }

        return new TruthTableRow(cells);
    }

    private static string FormatFunctionName(IReadOnlyList<string> outputNames)
    {
        return outputNames.Count switch
        {
            0 => "",
            1 => outputNames[0],
            _ => $"{outputNames[0]}-{outputNames[^1]}"
        };
    }
}
