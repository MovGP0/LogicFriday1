using System;
using System.Collections.ObjectModel;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.Views;

public partial class TruthTableSetupDialog : Window
{
    private const int MinimumInputCount = 2;
    private const int MaximumInputCount = 16;
    private const int MinimumOutputCount = 1;
    private const int MaximumOutputCount = 16;
    private static readonly Regex s_variableNameExpression = new("^[A-Za-z_][A-Za-z0-9_.\\[\\]]{0,7}$", RegexOptions.Compiled);

    private int _inputCount = 4;
    private int _outputCount = 1;

    public TruthTableSetupDialog()
    {
        InitializeComponent();
        DataContext = this;

        SetInputCount(_inputCount);
        SetOutputCount(_outputCount);
    }

    public ObservableCollection<TruthTableVariableName> Inputs { get; } = [];

    public ObservableCollection<TruthTableVariableName> Outputs { get; } = [];

    public string[] InputNames => Inputs.Select(item => item.Name.Trim()).ToArray();

    public string[] OutputNames => Outputs.Select(item => item.Name.Trim()).ToArray();

    public int InputCount
    {
        get => _inputCount;
        set => _inputCount = Clamp(value, MinimumInputCount, MaximumInputCount);
    }

    public int OutputCount
    {
        get => _outputCount;
        set => _outputCount = Clamp(value, MinimumOutputCount, MaximumOutputCount);
    }

    private void InputCount_OnValueChanged(object? sender, NumericUpDownValueChangedEventArgs e)
    {
        SetInputCount((int)(e.NewValue ?? _inputCount));
    }

    private void OutputCount_OnValueChanged(object? sender, NumericUpDownValueChangedEventArgs e)
    {
        SetOutputCount((int)(e.NewValue ?? _outputCount));
    }

    private async void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        var validationError = ValidateNames();
        if (validationError is not null)
        {
            await ShowMessageAsync(validationError);
            return;
        }

        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }

    private async void HelpButton_OnClick(object? sender, RoutedEventArgs e)
    {
        await ShowMessageAsync("Help contents are not available yet.");
    }

    private void SetInputCount(int count)
    {
        InputCount = count;

        while (Inputs.Count < _inputCount)
        {
            Inputs.Add(new TruthTableVariableName(GetDefaultInputName(Inputs.Count)));
        }

        while (Inputs.Count > _inputCount)
        {
            Inputs.RemoveAt(Inputs.Count - 1);
        }
    }

    private void SetOutputCount(int count)
    {
        OutputCount = count;

        while (Outputs.Count < _outputCount)
        {
            Outputs.Add(new TruthTableVariableName(GetDefaultOutputName(Outputs.Count)));
        }

        while (Outputs.Count > _outputCount)
        {
            Outputs.RemoveAt(Outputs.Count - 1);
        }
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

    private string? ValidateNames()
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

        if (names.Any(name => !s_variableNameExpression.IsMatch(name)))
        {
            return "Variable names may have only letters, digits, underscores, periods,\nand brackets. The name must begin with a letter or underscore.";
        }

        return null;
    }

    private async Task ShowMessageAsync(string message)
    {
        var dialog = new Window
        {
            Title = "Logic Friday",
            Width = 380,
            Height = 150,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            CanResize = false
        };

        var okButton = new Button
        {
            Content = "OK",
            MinWidth = 80,
            HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Right,
            IsDefault = true
        };

        okButton.Click += (_, _) => dialog.Close();

        dialog.Content = new Grid
        {
            RowDefinitions = new RowDefinitions("*,Auto"),
            Margin = new Avalonia.Thickness(16),
            Children =
            {
                new TextBlock
                {
                    Text = message,
                    TextWrapping = Avalonia.Media.TextWrapping.Wrap
                },
                okButton
            }
        };

        Grid.SetRow(okButton, 1);
        await dialog.ShowDialog(this);
    }

    private static int Clamp(int value, int minimum, int maximum)
    {
        return Math.Min(Math.Max(value, minimum), maximum);
    }
}

public partial class TruthTableVariableName(string name) : ObservableObject
{
    [ObservableProperty]
    private string _name = name;
}
