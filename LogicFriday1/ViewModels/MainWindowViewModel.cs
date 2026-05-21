using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.ViewModels;

public partial class MainWindowViewModel : ViewModelBase
{
    [ObservableProperty]
    private string _statusText = "Ready";

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(IsUnminimizedViewSelected))]
    private bool _isMinimizedViewSelected;

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
}
