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
    private FunctionSummaryRow? _selectedFunctionSummary;

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
        StatusText = "Entering new logic equation";
    }
}
