using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.Models;

public partial class TruthTableCell(string value, bool isOutput) : ObservableObject
{
    [ObservableProperty]
    private string _value = value;

    public bool IsOutput { get; } = isOutput;

    public void CycleOutputValue()
    {
        if (!IsOutput)
        {
            return;
        }

        Value = Value switch
        {
            "0" => "1",
            "1" => "X",
            _ => "0"
        };
    }
}
