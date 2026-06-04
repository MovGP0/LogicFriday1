using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.ViewModels;

public partial class GateTraceInputRow : ObservableObject
{
    public GateTraceInputRow(string name, int value)
    {
        Name = name;
        Value = value;
    }

    public string Name
    {
        get;
    }

    [ObservableProperty]
    private int _value;

    public string DisplayText => $"{Name} = {Value}";

    partial void OnValueChanged(int value)
    {
        OnPropertyChanged(nameof(DisplayText));
    }
}
