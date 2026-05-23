using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Interactivity;

namespace LogicFriday1.Views;

public partial class GateVariableNameDialog : Window
{
    public GateVariableNameDialog()
    {
        InitializeComponent();
        DataContext = this;

        Opened += (_, _) =>
        {
            VariableNameTextBox.Text = VariableName;
            VariableNameTextBox.Focus();
            VariableNameTextBox.SelectAll();
        };
    }

    public IEnumerable<string> ExistingNames { get; set; } = [];

    public string VariableName { get; set; } = string.Empty;

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        VariableName = VariableNameTextBox.Text ?? string.Empty;
        var validationError = ValidateName();
        if (validationError is not null)
        {
            ErrorTextBlock.Text = validationError;
            return;
        }

        VariableName = VariableName.Trim();
        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }

    private string? ValidateName()
    {
        var name = VariableName.Trim();
        if (name.Length is 0 or > 8)
        {
            return "Error: Name must contain 1 to 8 characters.";
        }

        var existingNames = ExistingNames
            .Select(static existingName => existingName.Trim())
            .Where(static existingName => existingName.Length > 0)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        if (existingNames.Contains(name))
        {
            return "Error: Name already in use.";
        }

        if (!IsVariableStart(name[0]))
        {
            return "Error: Name must begin with a letter or underscore.";
        }

        var illegalCharacter = name.FirstOrDefault(static character => !IsVariablePart(character));
        if (illegalCharacter != default)
        {
            return $"Illegal character: '{illegalCharacter}'";
        }

        return null;
    }

    private static bool IsVariableStart(char value)
    {
        return value is '_' || char.IsAsciiLetter(value);
    }

    private static bool IsVariablePart(char value)
    {
        return IsVariableStart(value) ||
            char.IsAsciiDigit(value) ||
            value is '.' or '[' or ']';
    }
}
