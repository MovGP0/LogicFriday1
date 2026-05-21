using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Models;

namespace LogicFriday1.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
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
    private FunctionSummaryRow? _selectedFunctionSummary;

    [ObservableProperty]
    private GatePaletteItem? _selectedGatePaletteItem;

    public ObservableCollection<TruthTableRow> TruthTableRows { get; } = [];

    public ObservableCollection<GatePaletteItem> GatePaletteItems { get; } =
    [
        new("Select", GatePaletteKind.Select, 0x42b, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("Wire", GatePaletteKind.Wire, 0x42c, 0, "Decompiled/logicfriday_decompiled_functions/0040cabd_FUN_0040cabd.c"),
        new("NOT", GatePaletteKind.Not, 0x3f4, 1, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
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
        new("0", GatePaletteKind.ConstantZero, 0x42a, 0, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("1", GatePaletteKind.ConstantOne, 0x408, 0, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
        new("XOR", GatePaletteKind.Xor, 0x430, 2, "Decompiled/logicfriday_decompiled_functions/0042af77_FUN_0042af77.c"),
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
        StatusText = "Entering new logic equation";
    }

    public void StartNewTruthTable(string[] inputNames, string[] outputNames)
    {
        TruthTableRows.Clear();

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
                cells.Add(new TruthTableCell("0", true));
            }

            TruthTableRows.Add(new TruthTableRow(cells));
        }

        IsEquationEditorVisible = false;
        IsTruthTableVisible = true;
        IsGateDiagramVisible = false;
        StatusText = $"Editing truth table: {inputNames.Length} inputs, {outputNames.Length} outputs";
    }

    public void StartNewGateDiagram()
    {
        IsEquationEditorVisible = false;
        IsTruthTableVisible = false;
        IsGateDiagramVisible = true;
        SelectedGatePaletteItem = null;
        StatusText = "Editing gate diagram";
    }

    public void SelectGatePaletteItem(GatePaletteItem item)
    {
        SelectedGatePaletteItem = item;
        StatusText = $"Selected gate diagram tool: {item.Label}";
    }
}
