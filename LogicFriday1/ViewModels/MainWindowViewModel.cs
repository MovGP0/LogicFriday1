using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Models;
using LogicFriday1.Services;

namespace LogicFriday1.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    private string[] _truthTableInputNames = [];
    private string[] _truthTableOutputNames = [];
    private FunctionSummaryRow? _truthTableEditTarget;

    [ObservableProperty]
    private string _statusText = "Ready";

    [ObservableProperty]
    private string _logicEquationText = "";

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    private bool _isEquationEditorVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    private bool _isTruthTableVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    private bool _isGateDiagramVisible;

    [ObservableProperty]
    private bool _isFunctionDetailVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    private FunctionSummaryRow? _selectedFunctionSummary;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    private int _selectedFunctionCount;

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

    public ObservableCollection<GateDiagramItem> GateDiagramItems { get; } = [];

    public ObservableCollection<GateDiagramWire> GateDiagramWires { get; } = [];

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

    public bool IsFunctionViewModeEnabled
    {
        get => SelectedFunctionSummary?.LogicFunction is not null &&
            !IsEquationEditorVisible &&
            !IsTruthTableVisible &&
            !IsGateDiagramVisible;
    }

    public bool IsMinimizedViewEnabled
    {
        get => IsFunctionViewModeEnabled && HasMinimizedView(SelectedFunctionSummary);
    }

    public bool IsOperationMinimizeEnabled
    {
        get => IsFunctionViewModeEnabled;
    }

    public bool IsTruthTableModifyEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1 &&
            SelectedFunctionSummary?.LogicFunction is TruthTableLogicFunction;
    }

    public void ShowUnminimizedView()
    {
        if (!IsFunctionViewModeEnabled)
        {
            StatusText = "No function is selected";
            return;
        }

        IsMinimizedViewSelected = false;
        NotifyFunctionViewModeChanged();
        if (SelectedFunctionSummary?.LogicFunction is { } logicFunction)
        {
            ShowFunction(logicFunction);
        }

        StatusText = "Showing unminimized function view";
    }

    public void ShowMinimizedView()
    {
        if (!IsMinimizedViewEnabled)
        {
            StatusText = "Minimized function view is not available";
            return;
        }

        IsMinimizedViewSelected = true;
        NotifyFunctionViewModeChanged();
        if (SelectedFunctionSummary?.LogicFunction is { } logicFunction)
        {
            ShowFunction(logicFunction);
        }

        StatusText = "Showing minimized function view";
    }

    public void MinimizeSelectedFunction()
    {
        if (!IsOperationMinimizeEnabled || SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            StatusText = "No function is selected";
            return;
        }

        try
        {
            var minimizedFunction = LogicFunctionMinimizer.Minimize(logicFunction);
            var updatedFunction = WithMinimizedFunction(logicFunction, minimizedFunction);
            var updatedSummary = CreateFunctionSummary(updatedFunction);
            var summaryIndex = FunctionSummaries.IndexOf(summary);
            IsMinimizedViewSelected = true;
            if (summaryIndex >= 0)
            {
                FunctionSummaries[summaryIndex] = updatedSummary;
            }

            SelectedFunctionSummary = updatedSummary;
            NotifyFunctionViewModeChanged();
            ShowFunction(updatedFunction);
            StatusText = $"Function minimized: {minimizedFunction.Products.Count} product terms";
        }
        catch (Exception ex)
        {
            StatusText = $"Minimize failed: {ex.Message}";
        }
    }

    public void StartNewLogicEquation()
    {
        LogicEquationText = "";
        _truthTableEditTarget = null;
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

    public bool StartModifyTruthTable()
    {
        if (!IsTruthTableModifyEnabled ||
            SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            StatusText = "No function is selected";
            return false;
        }

        StartTruthTable(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            logicFunction.OutputValues,
            "Modifying truth table",
            summary);
        return true;
    }

    private void StartTruthTable(
        string[] inputNames,
        string[] outputNames,
        IReadOnlyList<string[]> outputValues,
        string statusPrefix,
        FunctionSummaryRow? editTarget = null)
    {
        TruthTableRows.Clear();
        _truthTableEditTarget = editTarget;
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
        GateDiagramItems.Clear();
        GateDiagramWires.Clear();
        _truthTableEditTarget = null;
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

        try
        {
            var parsedEquation = LogicEquationParser.Parse(LogicEquationText);
            var logicFunction = new LogicEquationFunction(
                parsedEquation.InputNames,
                parsedEquation.OutputNames,
                parsedEquation.OutputValues,
                parsedEquation.EquationText);

            AddFunction(logicFunction);
            ShowFunction(logicFunction);
            StatusText = "Logic equation submitted";
        }
        catch (LogicEquationParseException ex)
        {
            StatusText = ex.Message;
        }
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
        _truthTableEditTarget = null;
        IsTruthTableVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Ready";
    }

    public void CancelGateDiagramEditing()
    {
        GateDiagramItems.Clear();
        GateDiagramWires.Clear();
        SelectedGatePaletteItem = null;
        IsGateDiagramVisible = false;
        StatusText = "Ready";
    }

    public bool SubmitGateDiagramEditing(out string? errorMessage)
    {
        errorMessage = null;
        if (!IsGateDiagramVisible)
        {
            errorMessage = "No gate diagram is active";
            StatusText = errorMessage;
            return false;
        }

        try
        {
            var conversion = GateDiagramConverter.Convert(GateDiagramItems, GateDiagramWires);
            var logicFunction = new GateDiagramFunction(
                conversion.InputNames,
                conversion.OutputNames,
                conversion.OutputValues,
                conversion.EquationText,
                GateDiagramItems.ToArray(),
                GateDiagramWires.ToArray());

            AddFunction(logicFunction);
            ShowFunction(logicFunction);
            SelectedGatePaletteItem = null;
            IsGateDiagramVisible = false;
            StatusText = "Gate diagram submitted";
            return true;
        }
        catch (GateDiagramConversionException ex)
        {
            errorMessage = ex.Message;
            StatusText = ex.Message;
            return false;
        }
    }

    public void SubmitTruthTableEditing()
    {
        if (_truthTableInputNames.Length == 0 || _truthTableOutputNames.Length == 0)
        {
            StatusText = "No truth table is active";
            return;
        }

        var logicFunction = CreateTruthTableFunction();
        var editTarget = _truthTableEditTarget;
        if (editTarget is not null)
        {
            ReplaceFunction(editTarget, logicFunction);
        }
        else
        {
            AddFunction(logicFunction);
        }

        ShowFunction(logicFunction);
        TruthTableRows.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        _truthTableEditTarget = null;
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        StatusText = editTarget is null ? "Truth table submitted" : "Truth table modified";
    }

    public void CloseCurrentDocument()
    {
        LogicEquationText = "";
        TruthTableRows.Clear();
        FunctionTruthTableRows.Clear();
        GateDiagramItems.Clear();
        GateDiagramWires.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        _truthTableEditTarget = null;
        SelectedGatePaletteItem = null;
        SelectedFunctionSummary = null;
        SelectedFunctionCount = 0;
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

    public void ClearGatePaletteSelection()
    {
        SelectedGatePaletteItem = null;
        StatusText = "Ready";
    }

    public void SetSelectedFunctionCount(int selectedFunctionCount)
    {
        SelectedFunctionCount = selectedFunctionCount;
    }

    public LogicFunction? GetSelectedFunction()
    {
        return SelectedFunctionSummary?.LogicFunction;
    }

    public void ShowFunction(LogicFunction logicFunction)
    {
        if (logicFunction.MinimizedFunction is null)
        {
            IsMinimizedViewSelected = false;
            NotifyFunctionViewModeChanged();
        }

        LogicEquationText = IsMinimizedViewSelected && logicFunction.MinimizedFunction is { } minimizedFunction
            ? minimizedFunction.EquationText
            : logicFunction.EquationText;

        if (IsMinimizedViewSelected && logicFunction.MinimizedFunction is not null)
        {
            RefreshMinimizedFunctionTruthTable(logicFunction);
        }
        else
        {
            RefreshFunctionTruthTable(logicFunction);
        }

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
        SelectedFunctionCount = 1;
    }

    private void ReplaceFunction(FunctionSummaryRow editTarget, LogicFunction logicFunction)
    {
        var summary = CreateFunctionSummary(logicFunction);
        var summaryIndex = FunctionSummaries.IndexOf(editTarget);
        if (summaryIndex < 0)
        {
            AddFunction(logicFunction);
            return;
        }

        FunctionSummaries[summaryIndex] = summary;
        SelectedFunctionSummary = summary;
        SelectedFunctionCount = 1;
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
            PI: logicFunction.MinimizedFunction is null
                ? "Unminimized"
                : logicFunction.MinimizedFunction.Products.Count.ToString(),
            Gates: "Not Mapped",
            LogicFunction: logicFunction);
    }

    private static bool HasMinimizedView(FunctionSummaryRow? summary)
    {
        return summary?.LogicFunction?.MinimizedFunction is not null;
    }

    private void NotifyFunctionViewModeChanged()
    {
        OnPropertyChanged(nameof(IsUnminimizedViewSelected));
        OnPropertyChanged(nameof(IsFunctionViewModeEnabled));
        OnPropertyChanged(nameof(IsMinimizedViewEnabled));
        OnPropertyChanged(nameof(IsOperationMinimizeEnabled));
        OnPropertyChanged(nameof(IsTruthTableModifyEnabled));
    }

    private static LogicFunction WithMinimizedFunction(
        LogicFunction logicFunction,
        MinimizedLogicFunction minimizedFunction)
    {
        return logicFunction switch
        {
            TruthTableLogicFunction truthTableFunction => truthTableFunction with
            {
                MinimizedFunction = minimizedFunction
            },
            LogicEquationFunction logicEquationFunction => logicEquationFunction with
            {
                MinimizedFunction = minimizedFunction
            },
            GateDiagramFunction gateDiagramFunction => gateDiagramFunction with
            {
                MinimizedFunction = minimizedFunction
            },
            _ => throw new InvalidOperationException("Unsupported function type.")
        };
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

    private void RefreshMinimizedFunctionTruthTable(LogicFunction logicFunction)
    {
        FunctionTruthTableRows.Clear();
        if (logicFunction.MinimizedFunction is null)
        {
            return;
        }

        for (var productIndex = 0; productIndex < logicFunction.MinimizedFunction.Products.Count; productIndex++)
        {
            FunctionTruthTableRows.Add(CreateMinimizedTruthTableRow(
                productIndex + 1,
                logicFunction.MinimizedFunction.Products[productIndex],
                outputCellsEditable: false));
        }
    }

    private static TruthTableRow CreateMinimizedTruthTableRow(
        int productIndex,
        MinimizedProductTerm product,
        bool outputCellsEditable)
    {
        var cells = new List<TruthTableCell>
        {
            new(productIndex.ToString(), false)
        };

        foreach (var inputValue in product.InputPattern)
        {
            cells.Add(new TruthTableCell(inputValue.ToString(), false));
        }

        cells.Add(new TruthTableCell("", false));

        foreach (var outputValue in product.OutputValues)
        {
            cells.Add(new TruthTableCell(outputValue, outputCellsEditable));
        }

        return new TruthTableRow(cells);
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
