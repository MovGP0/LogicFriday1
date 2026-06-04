using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Models;
using LogicFriday1.Services;

namespace LogicFriday1.ViewModels;

public partial class MainWindowViewModel : ObservableObject
{
    private string[] _truthTableInputNames = [];

    private string[] _truthTableOutputNames = [];

    private FunctionSummaryRow? _truthTableEditTarget;

    private FunctionSummaryRow? _logicEquationEditTarget;

    private readonly Dictionary<LogicFunction, bool> _showAllTruthTableRowsByFunction = [];

    private IReadOnlyList<FunctionSummaryRow> _selectedFunctionSummaries = [];

    private bool _isSettingSelectedFunctionSummaries;

    [ObservableProperty]
    private string _statusText = "Ready";

    [ObservableProperty]
    private string _logicEquationText = "";

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMapToGatesEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCloneFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCompareFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationTwoFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationOrFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationAndFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationXorFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationGenerateLookupFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationFormatEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationSubmitEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationCancelEnabled))]
    private bool _isEquationEditorVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMapToGatesEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCloneFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCompareFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationTwoFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationOrFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationAndFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationXorFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationGenerateLookupFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationFormatEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableSubmitEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableCancelEnabled))]
    private bool _isTruthTableVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMapToGatesEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCloneFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCompareFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationTwoFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationOrFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationAndFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationXorFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationGenerateLookupFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationFormatEnabled))]
    private bool _isGateDiagramVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsTruthTableShowModeEnabled))]
    private bool _isFunctionDetailVisible;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsFunctionViewModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsMinimizedViewEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMapToGatesEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCloneFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCompareFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationTwoFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationOrFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationAndFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationXorFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationGenerateLookupFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableShowModeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationFormatEnabled))]
    [NotifyPropertyChangedFor(nameof(IsShowAllTruthTableRowsSelected))]
    [NotifyPropertyChangedFor(nameof(IsShowTrueAndDontCareTruthTableRowsSelected))]
    private FunctionSummaryRow? _selectedFunctionSummary;

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsOperationMinimizeEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationMapToGatesEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCloneFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationCompareFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationTwoFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationOrFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationAndFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationXorFunctionsEnabled))]
    [NotifyPropertyChangedFor(nameof(IsOperationGenerateLookupFunctionEnabled))]
    [NotifyPropertyChangedFor(nameof(IsTruthTableModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationModifyEnabled))]
    [NotifyPropertyChangedFor(nameof(IsEquationFormatEnabled))]
    private int _selectedFunctionCount;

    [ObservableProperty]
    private GatePaletteItem? _selectedGatePaletteItem;

    public ObservableCollection<TruthTableRow> TruthTableRows { get; } = [];

    public ObservableCollection<TruthTableRow> FunctionTruthTableRows { get; } = [];

    public ObservableCollection<GatePaletteItem> GatePaletteItems
    {
        get;
    } =
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
    [NotifyPropertyChangedFor(nameof(IsTruthTableShowModeEnabled))]
    private bool _isMinimizedViewSelected;

    public ObservableCollection<FunctionSummaryRow> FunctionSummaries
    {
        get;
    } =
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

    public bool IsShowTrueAndDontCareTruthTableRowsSelected
    {
        get => !IsShowAllTruthTableRowsSelected;
    }

    public bool IsShowAllTruthTableRowsSelected
    {
        get => SelectedFunctionSummary?.LogicFunction is { } logicFunction &&
            _showAllTruthTableRowsByFunction.TryGetValue(logicFunction, out var showAllRows) &&
            showAllRows;
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
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1 &&
            SelectedFunctionSummary?.LogicFunction?.MinimizedFunction is null;
    }

    public bool IsOperationMapToGatesEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1;
    }

    public bool IsOperationGenerateLookupFunctionEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1;
    }

    public bool IsOperationCloneFunctionEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1;
    }

    public bool IsOperationCompareFunctionsEnabled
    {
        get => IsOperationTwoFunctionEnabled;
    }

    public bool IsOperationTwoFunctionEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 2;
    }

    public bool IsOperationOrFunctionsEnabled
    {
        get => IsOperationTwoFunctionEnabled;
    }

    public bool IsOperationAndFunctionsEnabled
    {
        get => IsOperationTwoFunctionEnabled;
    }

    public bool IsOperationXorFunctionsEnabled
    {
        get => IsOperationTwoFunctionEnabled;
    }

    public bool IsOperationCancelEnabled
    {
        get => false;
    }

    public bool IsTruthTableModifyEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1 &&
            SelectedFunctionSummary?.LogicFunction is TruthTableLogicFunction;
    }

    public bool IsEquationModifyEnabled
    {
        get => IsFunctionViewModeEnabled &&
            SelectedFunctionCount == 1 &&
            SelectedFunctionSummary?.LogicFunction is not null;
    }

    public bool IsEquationFormatEnabled
    {
        get => IsEquationModifyEnabled;
    }

    public bool IsEquationSubmitEnabled
    {
        get => IsEquationEditorVisible;
    }

    public bool IsEquationCancelEnabled
    {
        get => IsEquationEditorVisible;
    }

    public bool IsTruthTableSubmitEnabled
    {
        get => IsTruthTableVisible;
    }

    public bool IsTruthTableCancelEnabled
    {
        get => IsTruthTableVisible;
    }

    public bool IsTruthTableShowModeEnabled
    {
        get => IsFunctionDetailVisible &&
            !IsMinimizedViewSelected &&
            SelectedFunctionSummary?.LogicFunction is not null;
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

    public void ShowTrueAndDontCareTruthTableRows()
    {
        if (!IsTruthTableShowModeEnabled)
        {
            StatusText = "Truth table row view is not available";
            return;
        }

        SetShowAllTruthTableRows(false);
        StatusText = "Showing true and don't care truth table rows";
    }

    public void ShowAllTruthTableRows()
    {
        if (!IsTruthTableShowModeEnabled)
        {
            StatusText = "Truth table row view is not available";
            return;
        }

        SetShowAllTruthTableRows(true);
        StatusText = "Showing all truth table rows";
    }

    public void MinimizeSelectedFunction(MinimizeOptions options)
    {
        if (!IsOperationMinimizeEnabled || SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            StatusText = "No function is selected";
            return;
        }

        try
        {
            var minimizedFunction = LogicFunctionMinimizer.Minimize(logicFunction, options);
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

    public bool MapSelectedFunctionToGates(
        MapToGatesDialogViewModel options,
        out string? errorMessage)
    {
        errorMessage = null;
        if (!IsOperationMapToGatesEnabled ||
            SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            errorMessage = "No function is selected";
            StatusText = errorMessage;
            return false;
        }

        try
        {
            var mappedFunction = LogicFunctionGateMapper.Map(logicFunction, options);
            var mappedSummary = CreateFunctionSummary(mappedFunction) with
            {
                Gates = mappedFunction.Items
                    .Count(static item => item.Kind is not GatePaletteKind.Input and not GatePaletteKind.Output)
                    .ToString()
            };
            var summaryIndex = FunctionSummaries.IndexOf(summary);
            if (summaryIndex >= 0)
            {
                FunctionSummaries[summaryIndex] = mappedSummary;
            }
            else
            {
                FunctionSummaries.Add(mappedSummary);
            }

            SelectedFunctionSummary = mappedSummary;
            SelectedFunctionCount = 1;
            ShowGateDiagramFunction(mappedFunction);
            StatusText = $"Mapped to gates: {mappedSummary.Gates} gates";
            return true;
        }
        catch (Exception ex)
        {
            errorMessage = $"Map to Gates failed: {ex.Message}";
            StatusText = errorMessage;
            return false;
        }
    }

    public bool CloneSelectedFunction()
    {
        if (!IsOperationCloneFunctionEnabled ||
            SelectedFunctionSummary is not { LogicFunction: { } logicFunction })
        {
            StatusText = "Select one function to clone";
            return false;
        }

        var clone = CloneLogicFunction(logicFunction);
        AddFunction(clone);
        ShowFunction(clone);
        StatusText = "Function cloned";
        return true;
    }

    public bool CompareSelectedFunctions()
    {
        if (!IsOperationCompareFunctionsEnabled)
        {
            StatusText = "Select two functions to compare";
            return false;
        }

        var selectedFunctions = _selectedFunctionSummaries
            .Select(static summary => summary.LogicFunction)
            .OfType<LogicFunction>()
            .ToArray();
        if (selectedFunctions.Length != 2)
        {
            StatusText = "Select two functions to compare";
            return false;
        }

        var firstFunction = selectedFunctions[0];
        var secondFunction = selectedFunctions[1];
        if (!CanApplyTwoFunctionOperation(firstFunction, secondFunction))
        {
            StatusText = "Functions must have the same inputs and exactly one output.";
            return false;
        }

        var functionsAreEquivalent = firstFunction.OutputValues
            .Zip(secondFunction.OutputValues)
            .All(static pair => pair.First[0] == pair.Second[0]);
        StatusText = functionsAreEquivalent
            ? "Functions are equivalent"
            : "Functions are different";
        return true;
    }

    public bool OrSelectedFunctions(out string? errorMessage)
    {
        return ApplyTwoFunctionOperation(
            "OR",
            CombineOrValues,
            out errorMessage);
    }

    public bool AndSelectedFunctions(out string? errorMessage)
    {
        return ApplyTwoFunctionOperation(
            "AND",
            CombineAndValues,
            out errorMessage);
    }

    public bool XorSelectedFunctions(out string? errorMessage)
    {
        return ApplyTwoFunctionOperation(
            "XOR",
            CombineXorValues,
            out errorMessage);
    }

    public void CancelOperation()
    {
        StatusText = "No operation is active";
    }

    public void StartNewLogicEquation()
    {
        LogicEquationText = "";
        _truthTableEditTarget = null;
        _logicEquationEditTarget = null;
        IsEquationEditorVisible = true;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Entering new logic equation";
    }

    public bool StartModifyLogicEquation()
    {
        if (!IsEquationModifyEnabled ||
            SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            StatusText = "No function is selected";
            return false;
        }

        LogicEquationText = logicFunction.EquationText;
        _truthTableEditTarget = null;
        _logicEquationEditTarget = summary;
        IsEquationEditorVisible = true;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = false;
        IsFunctionDetailVisible = false;
        StatusText = "Modifying logic equation";
        return true;
    }

    public void ShowSumOfProductsEquation()
    {
        UpdateSelectedFunctionEquation(
            static viewModel => viewModel.GenerateSelectedSumOfProductsEquation(),
            "Setting sum of products equation");
    }

    public void ShowProductOfSumsEquation()
    {
        UpdateSelectedFunctionEquation(
            static viewModel => viewModel.GenerateSelectedProductOfSumsEquation(),
            "Setting product of sums equation");
    }

    public void FactorSelectedEquation()
    {
        UpdateSelectedFunctionEquation(
            static viewModel => string.Join(
                Environment.NewLine,
                "Factored:",
                viewModel.GenerateSelectedSumOfProductsEquation()),
            "Factored equation");
    }

    private void UpdateSelectedFunctionEquation(
        Func<MainWindowViewModel, string> equationFactory,
        string statusText)
    {
        if (!IsEquationFormatEnabled ||
            SelectedFunctionSummary is not { LogicFunction: { } logicFunction } summary)
        {
            StatusText = "No function is selected";
            return;
        }

        var updatedFunction = WithEquationText(logicFunction, equationFactory(this));
        var updatedSummary = CreateFunctionSummary(updatedFunction);
        var summaryIndex = FunctionSummaries.IndexOf(summary);
        if (summaryIndex >= 0)
        {
            FunctionSummaries[summaryIndex] = updatedSummary;
        }
        else
        {
            FunctionSummaries.Add(updatedSummary);
        }

        SelectedFunctionSummary = updatedSummary;
        SelectedFunctionCount = 1;
        ShowFunction(updatedFunction);
        StatusText = statusText;
    }

    private string GenerateSelectedSumOfProductsEquation()
    {
        if (SelectedFunctionSummary?.LogicFunction is not { } logicFunction)
        {
            return "";
        }

        return GenerateSumOfProductsEquation(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            logicFunction.OutputValues,
            "Sum of Products:");
    }

    private string GenerateSelectedProductOfSumsEquation()
    {
        if (SelectedFunctionSummary?.LogicFunction is not { } logicFunction)
        {
            return "";
        }

        return GenerateProductOfSumsEquation(
            logicFunction.InputNames,
            logicFunction.OutputNames,
            logicFunction.OutputValues,
            "Product of Sums:");
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
        _logicEquationEditTarget = null;
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
        _logicEquationEditTarget = null;
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

            var editTarget = _logicEquationEditTarget;
            if (editTarget is not null)
            {
                ReplaceFunction(editTarget, logicFunction);
            }
            else
            {
                AddFunction(logicFunction);
            }

            _logicEquationEditTarget = null;
            ShowFunction(logicFunction);
            StatusText = editTarget is null ? "Logic equation submitted" : "Logic equation modified";
        }
        catch (LogicEquationParseException ex)
        {
            StatusText = ex.Message;
        }
    }

    public void CancelLogicEquationEditing()
    {
        var selectedFunction = SelectedFunctionSummary?.LogicFunction;
        LogicEquationText = "";
        _logicEquationEditTarget = null;
        IsEquationEditorVisible = false;

        if (selectedFunction is not null)
        {
            ShowFunction(selectedFunction);
        }

        StatusText = "Ready";
    }

    public void CancelTruthTableEditing()
    {
        if (!IsTruthTableVisible)
        {
            return;
        }

        var selectedFunction = SelectedFunctionSummary?.LogicFunction;
        TruthTableRows.Clear();
        _truthTableInputNames = [];
        _truthTableOutputNames = [];
        _truthTableEditTarget = null;
        IsTruthTableVisible = false;

        if (selectedFunction is not null)
        {
            ShowFunction(selectedFunction);
        }
        else
        {
            IsFunctionDetailVisible = false;
        }

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
        _logicEquationEditTarget = null;
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

    public void SetSelectedFunctionSummaries(IReadOnlyList<FunctionSummaryRow> selectedFunctionSummaries)
    {
        var summaries = selectedFunctionSummaries
            .Where(static summary => summary.LogicFunction is not null)
            .ToArray();

        _isSettingSelectedFunctionSummaries = true;
        try
        {
            _selectedFunctionSummaries = summaries;
            SelectedFunctionSummary = summaries.FirstOrDefault();
            SelectedFunctionCount = summaries.Length;
        }
        finally
        {
            _isSettingSelectedFunctionSummaries = false;
        }

        NotifySelectionChanged();
    }

    public LogicFunction? GetSelectedFunction()
    {
        return SelectedFunctionSummary?.LogicFunction;
    }

    public void ShowFunction(LogicFunction logicFunction)
    {
        if (logicFunction is GateDiagramFunction gateDiagramFunction)
        {
            ShowGateDiagramFunction(gateDiagramFunction);
            return;
        }

        if (logicFunction.MinimizedFunction is null)
        {
            IsMinimizedViewSelected = false;
            NotifyFunctionViewModeChanged();
        }

        LogicEquationText = IsMinimizedViewSelected && logicFunction.MinimizedFunction is { } minimizedFunction
            ? AppendMinimizedEquationText(logicFunction.EquationText, minimizedFunction.EquationText)
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
        OnPropertyChanged(nameof(IsTruthTableShowModeEnabled));
        StatusText = $"Showing {logicFunction.OutputNames.Length} output function";
    }

    private void ShowGateDiagramFunction(GateDiagramFunction logicFunction)
    {
        if (logicFunction.MinimizedFunction is null)
        {
            IsMinimizedViewSelected = false;
            NotifyFunctionViewModeChanged();
        }

        LogicEquationText = logicFunction.EquationText;
        RefreshFunctionTruthTable(logicFunction);
        GateDiagramItems.Clear();
        foreach (var item in logicFunction.Items)
        {
            GateDiagramItems.Add(item);
        }

        GateDiagramWires.Clear();
        foreach (var wire in logicFunction.Wires)
        {
            GateDiagramWires.Add(wire);
        }

        SelectedGatePaletteItem = null;
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = true;
        IsFunctionDetailVisible = false;
        OnPropertyChanged(nameof(IsTruthTableShowModeEnabled));
        StatusText = $"Showing mapped gate diagram for {logicFunction.OutputNames.Length} output function";
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
                .Select((outputs, term) => (outputs, term))
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

    private static string GenerateProductOfSumsEquation(
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
            var falseTerms = outputValues
                .Select((outputs, term) => (outputs, term))
                .Where(row => row.outputs[outputIndex] == "0")
                .ToArray();

            if (falseTerms.Length == outputValues.Count)
            {
                equations.Add($"{outputNames[outputIndex]} = 0;");
            }
            else if (falseTerms.Length == 0)
            {
                equations.Add($"{outputNames[outputIndex]} = 1;");
            }
            else
            {
                var terms = falseTerms.Select(row => BuildSumTerm(row.term, inputNames));
                equations.Add($"{outputNames[outputIndex]} = {string.Join(" ", terms)};");
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

    private static string BuildSumTerm(int term, string[] inputNames)
    {
        var literals = new List<string>();
        for (var inputIndex = 0; inputIndex < inputNames.Length; inputIndex++)
        {
            var bitOffset = inputNames.Length - inputIndex - 1;
            var inputValue = (term >> bitOffset) & 1;
            literals.Add(inputValue == 0
                ? inputNames[inputIndex]
                : $"{inputNames[inputIndex]}'");
        }

        return $"({string.Join(" + ", literals)})";
    }

    private bool ApplyTwoFunctionOperation(
        string operationName,
        Func<string, string, string> combineValues,
        out string? errorMessage)
    {
        errorMessage = null;
        if (!IsOperationTwoFunctionEnabled)
        {
            errorMessage = "Please select two functions to compare.";
            StatusText = errorMessage;
            return false;
        }

        var selectedFunctions = _selectedFunctionSummaries
            .Select(static summary => summary.LogicFunction)
            .OfType<LogicFunction>()
            .ToArray();
        if (selectedFunctions.Length != 2)
        {
            errorMessage = "Please select two functions to compare.";
            StatusText = errorMessage;
            return false;
        }

        var firstFunction = selectedFunctions[0];
        var secondFunction = selectedFunctions[1];
        if (!CanApplyTwoFunctionOperation(firstFunction, secondFunction))
        {
            errorMessage = "Functions must have the same inputs and exactly one output.";
            StatusText = errorMessage;
            return false;
        }

        var outputName = $"{firstFunction.OutputNames[0]}_{operationName}_{secondFunction.OutputNames[0]}";
        var outputValues = firstFunction.OutputValues
            .Zip(
                secondFunction.OutputValues,
                (firstOutputs, secondOutputs) => new[]
                {
                    combineValues(firstOutputs[0], secondOutputs[0])
                })
            .ToArray();
        var resultFunction = new TruthTableLogicFunction(
            firstFunction.InputNames.ToArray(),
            [outputName],
            outputValues,
            GenerateSumOfProductsEquation(
                firstFunction.InputNames,
                [outputName],
                outputValues,
                $"{operationName} Functions:"));

        AddFunction(resultFunction);
        ShowFunction(resultFunction);
        StatusText = $"{operationName} Functions created";
        return true;
    }

    private static bool CanApplyTwoFunctionOperation(
        LogicFunction firstFunction,
        LogicFunction secondFunction)
    {
        return firstFunction.OutputNames.Length == 1 &&
            secondFunction.OutputNames.Length == 1 &&
            firstFunction.InputNames.SequenceEqual(secondFunction.InputNames) &&
            firstFunction.OutputValues.Count == secondFunction.OutputValues.Count;
    }

    private static string CombineOrValues(string firstValue, string secondValue)
    {
        if (firstValue == "1" || secondValue == "1")
        {
            return "1";
        }

        return firstValue == "X" || secondValue == "X"
            ? "X"
            : "0";
    }

    private static string CombineAndValues(string firstValue, string secondValue)
    {
        if (firstValue == "0" || secondValue == "0")
        {
            return "0";
        }

        return firstValue == "X" || secondValue == "X"
            ? "X"
            : "1";
    }

    private static string CombineXorValues(string firstValue, string secondValue)
    {
        if (firstValue == "X" || secondValue == "X")
        {
            return "X";
        }

        return firstValue == secondValue
            ? "0"
            : "1";
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
            Gates: "Not mapped",
            LogicFunction: logicFunction);
    }

    private static bool HasMinimizedView(FunctionSummaryRow? summary)
    {
        return summary?.LogicFunction?.MinimizedFunction is not null;
    }

    private static LogicFunction CloneLogicFunction(LogicFunction logicFunction)
    {
        return logicFunction switch
        {
            TruthTableLogicFunction truthTableFunction => truthTableFunction with
            {
                InputNames = CloneArray(truthTableFunction.InputNames),
                OutputNames = CloneArray(truthTableFunction.OutputNames),
                OutputValues = CloneRows(truthTableFunction.OutputValues),
                MinimizedFunction = CloneMinimizedFunction(truthTableFunction.MinimizedFunction)
            },
            LogicEquationFunction logicEquationFunction => logicEquationFunction with
            {
                InputNames = CloneArray(logicEquationFunction.InputNames),
                OutputNames = CloneArray(logicEquationFunction.OutputNames),
                OutputValues = CloneRows(logicEquationFunction.OutputValues),
                MinimizedFunction = CloneMinimizedFunction(logicEquationFunction.MinimizedFunction)
            },
            GateDiagramFunction gateDiagramFunction => gateDiagramFunction with
            {
                InputNames = CloneArray(gateDiagramFunction.InputNames),
                OutputNames = CloneArray(gateDiagramFunction.OutputNames),
                OutputValues = CloneRows(gateDiagramFunction.OutputValues),
                Items = gateDiagramFunction.Items.ToArray(),
                Wires = gateDiagramFunction.Wires.Select(CloneWire).ToArray(),
                MinimizedFunction = CloneMinimizedFunction(gateDiagramFunction.MinimizedFunction)
            },
            _ => throw new InvalidOperationException("Unsupported function type.")
        };
    }

    private static string[] CloneArray(string[] values)
    {
        return values.ToArray();
    }

    private static string[][] CloneRows(IReadOnlyList<string[]> rows)
    {
        return rows
            .Select(static row => row.ToArray())
            .ToArray();
    }

    private static MinimizedLogicFunction? CloneMinimizedFunction(MinimizedLogicFunction? minimizedFunction)
    {
        if (minimizedFunction is null)
        {
            return null;
        }

        return minimizedFunction with
        {
            Products = minimizedFunction.Products
                .Select(CloneMinimizedProductTerm)
                .ToArray()
        };
    }

    private static MinimizedProductTerm CloneMinimizedProductTerm(MinimizedProductTerm product)
    {
        return product with
        {
            OutputValues = product.OutputValues.ToArray()
        };
    }

    private static GateDiagramWire CloneWire(GateDiagramWire wire)
    {
        return wire with
        {
            RoutePoints = wire.RoutePoints.ToArray()
        };
    }

    private static LogicFunction WithEquationText(
        LogicFunction logicFunction,
        string equationText)
    {
        return logicFunction switch
        {
            TruthTableLogicFunction truthTableFunction => truthTableFunction with
            {
                EquationText = equationText,
                MinimizedFunction = null
            },
            LogicEquationFunction logicEquationFunction => logicEquationFunction with
            {
                EquationText = equationText,
                MinimizedFunction = null
            },
            GateDiagramFunction gateDiagramFunction => gateDiagramFunction with
            {
                EquationText = equationText,
                MinimizedFunction = null
            },
            _ => throw new InvalidOperationException("Unsupported function type.")
        };
    }

    partial void OnSelectedFunctionSummaryChanged(FunctionSummaryRow? value)
    {
        if (_isSettingSelectedFunctionSummaries)
        {
            return;
        }

        _selectedFunctionSummaries = value is null
            ? []
            : [value];
    }

    partial void OnSelectedFunctionCountChanged(int value)
    {
        if (_isSettingSelectedFunctionSummaries)
        {
            return;
        }

        if (value == 0)
        {
            _selectedFunctionSummaries = [];
        }
        else if (value == 1 && SelectedFunctionSummary is not null)
        {
            _selectedFunctionSummaries = [SelectedFunctionSummary];
        }
        else if (_selectedFunctionSummaries.Count != value)
        {
            _selectedFunctionSummaries = [];
        }
    }

    partial void OnIsEquationEditorVisibleChanged(bool value)
    {
        NotifyTwoFunctionOperationChanged();
    }

    partial void OnIsTruthTableVisibleChanged(bool value)
    {
        NotifyTwoFunctionOperationChanged();
    }

    partial void OnIsGateDiagramVisibleChanged(bool value)
    {
        NotifyTwoFunctionOperationChanged();
    }

    private void NotifyFunctionViewModeChanged()
    {
        OnPropertyChanged(nameof(IsUnminimizedViewSelected));
        OnPropertyChanged(nameof(IsFunctionViewModeEnabled));
        OnPropertyChanged(nameof(IsMinimizedViewEnabled));
        OnPropertyChanged(nameof(IsOperationMinimizeEnabled));
        OnPropertyChanged(nameof(IsOperationMapToGatesEnabled));
        OnPropertyChanged(nameof(IsOperationCloneFunctionEnabled));
        OnPropertyChanged(nameof(IsOperationCompareFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationTwoFunctionEnabled));
        OnPropertyChanged(nameof(IsOperationOrFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationAndFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationXorFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationGenerateLookupFunctionEnabled));
        OnPropertyChanged(nameof(IsTruthTableModifyEnabled));
        OnPropertyChanged(nameof(IsEquationModifyEnabled));
        OnPropertyChanged(nameof(IsEquationFormatEnabled));
        OnPropertyChanged(nameof(IsEquationSubmitEnabled));
        OnPropertyChanged(nameof(IsEquationCancelEnabled));
        OnPropertyChanged(nameof(IsTruthTableShowModeEnabled));
        OnPropertyChanged(nameof(IsShowAllTruthTableRowsSelected));
        OnPropertyChanged(nameof(IsShowTrueAndDontCareTruthTableRowsSelected));
    }

    private void NotifySelectionChanged()
    {
        OnPropertyChanged(nameof(IsFunctionViewModeEnabled));
        OnPropertyChanged(nameof(IsMinimizedViewEnabled));
        OnPropertyChanged(nameof(IsOperationMinimizeEnabled));
        OnPropertyChanged(nameof(IsOperationMapToGatesEnabled));
        OnPropertyChanged(nameof(IsOperationCloneFunctionEnabled));
        OnPropertyChanged(nameof(IsOperationCompareFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationGenerateLookupFunctionEnabled));
        OnPropertyChanged(nameof(IsTruthTableModifyEnabled));
        OnPropertyChanged(nameof(IsTruthTableShowModeEnabled));
        OnPropertyChanged(nameof(IsEquationModifyEnabled));
        OnPropertyChanged(nameof(IsEquationFormatEnabled));
        OnPropertyChanged(nameof(IsShowAllTruthTableRowsSelected));
        OnPropertyChanged(nameof(IsShowTrueAndDontCareTruthTableRowsSelected));
        NotifyTwoFunctionOperationChanged();
    }

    private void NotifyTwoFunctionOperationChanged()
    {
        OnPropertyChanged(nameof(IsOperationTwoFunctionEnabled));
        OnPropertyChanged(nameof(IsOperationOrFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationAndFunctionsEnabled));
        OnPropertyChanged(nameof(IsOperationXorFunctionsEnabled));
    }

    private void SetShowAllTruthTableRows(bool showAllRows)
    {
        if (SelectedFunctionSummary?.LogicFunction is not { } logicFunction)
        {
            return;
        }

        _showAllTruthTableRowsByFunction[logicFunction] = showAllRows;
        RefreshSelectedFunctionTruthTable();
        OnPropertyChanged(nameof(IsShowAllTruthTableRowsSelected));
        OnPropertyChanged(nameof(IsShowTrueAndDontCareTruthTableRowsSelected));
    }

    private void RefreshSelectedFunctionTruthTable()
    {
        if (SelectedFunctionSummary?.LogicFunction is not { } logicFunction)
        {
            return;
        }

        if (IsMinimizedViewSelected && logicFunction.MinimizedFunction is not null)
        {
            RefreshMinimizedFunctionTruthTable(logicFunction);
        }
        else
        {
            RefreshFunctionTruthTable(logicFunction);
        }
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

    private static string AppendMinimizedEquationText(
        string equationText,
        string minimizedEquationText)
    {
        if (string.IsNullOrWhiteSpace(equationText))
        {
            return minimizedEquationText;
        }

        return string.Join(
            Environment.NewLine,
            equationText.TrimEnd(),
            "",
            minimizedEquationText);
    }

    private void RefreshFunctionTruthTable(LogicFunction logicFunction)
    {
        FunctionTruthTableRows.Clear();

        for (var term = 0; term < logicFunction.OutputValues.Count; term++)
        {
            var outputs = logicFunction.OutputValues[term];
            if (!IsShowAllTruthTableRowsSelected &&
                outputs.All(static value => value == "0"))
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
