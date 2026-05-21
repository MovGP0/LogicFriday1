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
    private FunctionSummaryRow? _selectedFunctionSummary;

    public ObservableCollection<TruthTableRow> TruthTableRows { get; } = [];

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
        StatusText = $"Editing truth table: {inputNames.Length} inputs, {outputNames.Length} outputs";
    }
}
