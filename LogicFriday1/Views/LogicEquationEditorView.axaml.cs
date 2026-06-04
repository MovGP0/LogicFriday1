using System.ComponentModel;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;

namespace LogicFriday1.Views;

public partial class LogicEquationEditorView : UserControl
{
    public LogicEquationEditorView()
    {
        InitializeComponent();
    }

    public event EventHandler? SubmitRequested;

    public event EventHandler? CancelRequested;

    public event EventHandler? StateChanged;

    public bool HasSelection => EquationEditor.SelectionStart != EquationEditor.SelectionEnd;

    public bool HasText => !string.IsNullOrEmpty(EquationEditor.Text);

    public bool CanUndo => EquationEditor.CanUndo;

    public bool CanRedo => EquationEditor.CanRedo;

    public void FocusEditor()
    {
        EquationEditor.Focus();
    }

    public void Undo()
    {
        EquationEditor.Undo();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Redo()
    {
        EquationEditor.Redo();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Cut()
    {
        EquationEditor.Cut();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Copy()
    {
        EquationEditor.Copy();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Paste()
    {
        EquationEditor.Paste();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void DeleteSelection()
    {
        EquationEditor.SelectedText = "";
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public void SelectAllText()
    {
        EquationEditor.SelectAll();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    public async Task<bool> CanPasteAsync()
    {
        try
        {
            var clipboard = TopLevel.GetTopLevel(EquationEditor)?.Clipboard;
            return clipboard is not null &&
                !string.IsNullOrEmpty(await clipboard.TryGetTextAsync());
        }
        catch
        {
            return false;
        }
    }

    private async void EquationEditorContextMenu_OnOpening(object? sender, CancelEventArgs e)
    {
        var hasSelection = HasSelection;
        EquationEditorUndoMenuItem.IsEnabled = EquationEditor.CanUndo;
        EquationEditorRedoMenuItem.IsEnabled = EquationEditor.CanRedo;
        EquationEditorCutMenuItem.IsEnabled = hasSelection;
        EquationEditorCopyMenuItem.IsEnabled = hasSelection;
        EquationEditorDeleteMenuItem.IsEnabled = hasSelection;
        EquationEditorSelectAllMenuItem.IsEnabled = HasText;
        EquationEditorPasteMenuItem.IsEnabled = await CanPasteAsync();
    }

    private void EquationEditorUndo_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        Undo();
    }

    private void EquationEditorRedo_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        Redo();
    }

    private void EquationEditorCut_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        Cut();
    }

    private void EquationEditorCopy_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        Copy();
    }

    private void EquationEditorPaste_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        Paste();
    }

    private void EquationEditorDelete_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        DeleteSelection();
    }

    private void EquationEditorSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        FocusEditor();
        SelectAllText();
    }

    private void EquationEditorSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitRequested?.Invoke(this, EventArgs.Empty);
    }

    private void EquationEditorCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        CancelRequested?.Invoke(this, EventArgs.Empty);
    }

    private void EquationEditor_OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            CancelRequested?.Invoke(this, EventArgs.Empty);
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Enter)
        {
            return;
        }

        var keyModifiers = e.KeyModifiers;
        if (keyModifiers.HasFlag(KeyModifiers.Control))
        {
            return;
        }

        if (keyModifiers.HasFlag(KeyModifiers.Shift) || keyModifiers.HasFlag(KeyModifiers.Alt))
        {
            e.Handled = true;
            return;
        }

        SubmitRequested?.Invoke(this, EventArgs.Empty);
        e.Handled = true;
    }

    private void EquationEditor_OnKeyUp(object? sender, KeyEventArgs e)
    {
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private void EquationEditor_OnPointerReleased(object? sender, PointerReleasedEventArgs e)
    {
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private void EquationEditor_OnTextChanged(object? sender, TextChangedEventArgs e)
    {
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private void EquationEditor_OnGotFocus(object? sender, RoutedEventArgs e)
    {
        StateChanged?.Invoke(this, EventArgs.Empty);
    }
}
