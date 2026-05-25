using System.Collections.ObjectModel;
using System.Text.RegularExpressions;
using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.ViewModels;

public partial class TruthTableSetupDialogViewModel : ObservableObject
{
    private const int MinimumInputCount = 2;
    private const int MaximumInputCount = 16;
    private const int MinimumOutputCount = 1;
    private const int MaximumOutputCount = 16;

    [ObservableProperty]
    private int _inputCount = 4;

    [ObservableProperty]
    private int _outputCount = 1;

    public TruthTableSetupDialogViewModel()
    {
        SetInputCount(InputCount);
        SetOutputCount(OutputCount);
    }

    public ObservableCollection<TruthTableVariableName> Inputs { get; } = [];

    public ObservableCollection<TruthTableVariableName> Outputs { get; } = [];

    public string[] InputNames => Inputs.Select(item => item.Name.Trim()).ToArray();

    public string[] OutputNames => Outputs.Select(item => item.Name.Trim()).ToArray();

    public void SetInputCount(int count)
    {
        InputCount = Clamp(count, MinimumInputCount, MaximumInputCount);

        while (Inputs.Count < InputCount)
        {
            Inputs.Add(new TruthTableVariableName(GetDefaultInputName(Inputs.Count)));
        }

        while (Inputs.Count > InputCount)
        {
            Inputs.RemoveAt(Inputs.Count - 1);
        }
    }

    public void SetOutputCount(int count)
    {
        OutputCount = Clamp(count, MinimumOutputCount, MaximumOutputCount);

        while (Outputs.Count < OutputCount)
        {
            Outputs.Add(new TruthTableVariableName(GetDefaultOutputName(Outputs.Count)));
        }

        while (Outputs.Count > OutputCount)
        {
            Outputs.RemoveAt(Outputs.Count - 1);
        }
    }

    public string? ValidateNames()
    {
        var names = Inputs.Concat(Outputs).Select(item => item.Name.Trim()).ToList();
        if (names.Any(name => name.Length is 0 or > 8))
        {
            return "Variable names must have between 1 and 8 characters.";
        }

        if (names.GroupBy(name => name, StringComparer.OrdinalIgnoreCase).Any(group => group.Count() > 1))
        {
            return "Variable name already in use.";
        }

        if (names.Any(name => !VariableNameExpression().IsMatch(name)))
        {
            return "Variable names may have only letters, digits, underscores, periods,\nand brackets. The name must begin with a letter or underscore.";
        }

        return null;
    }

    private static string GetDefaultInputName(int index)
    {
        const string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        return alphabet[index].ToString();
    }

    private static string GetDefaultOutputName(int index)
    {
        return $"F{index}";
    }

    private static int Clamp(int value, int minimum, int maximum)
    {
        return Math.Min(Math.Max(value, minimum), maximum);
    }

    [GeneratedRegex(@"^[A-Za-z_][A-Za-z0-9_.\[\]]{0,7}$", RegexOptions.Compiled)]
    private static partial Regex VariableNameExpression();
}

public partial class TruthTableVariableName(string name) : ObservableObject
{
    [ObservableProperty]
    private string _name = name;
}
