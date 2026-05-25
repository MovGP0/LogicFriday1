using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.ViewModels;

public partial class GateVariableNameDialogViewModel : ObservableObject
{
    [ObservableProperty]
    private string _variableName = string.Empty;

    [ObservableProperty]
    private string _errorText = string.Empty;

    public IEnumerable<string> ExistingNames { get; set; } = [];

    public bool TryAccept()
    {
        var validationError = ValidateName();
        if (validationError is not null)
        {
            ErrorText = validationError;
            return false;
        }

        VariableName = VariableName.Trim();
        ErrorText = string.Empty;
        return true;
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
        if (illegalCharacter != 0)
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
