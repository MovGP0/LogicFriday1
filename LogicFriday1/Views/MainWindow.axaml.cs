using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Data;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using LogicFriday1.Controls;
using LogicFriday1.Models;
using LogicFriday1.Services;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MainWindow : Window
{
    private const string HelpContentsUrl = "https://github.com/MovGP0/LogicFriday1/wiki";
    private const string GateDiagramHelpUrl = "https://github.com/MovGP0/LogicFriday1/wiki/Entering-a-gate-diagram";
    private const string ActiveGatePaletteButtonClass = "active";
    private TruthTableRow? _truthTableContextRow;
    private Button? _activeGatePaletteButton;

    public MainWindow()
    {
        InitializeComponent();
        TruthTableDataGrid.AddHandler(PointerPressedEvent, TruthTableDataGrid_OnPointerPressed, RoutingStrategies.Tunnel);
        GateDiagramSurface.VariableNameRequested += GateDiagramSurface_OnVariableNameRequested;
        GateDiagramSurface.PaletteSelectionCleared += GateDiagramSurface_OnPaletteSelectionCleared;
    }

    private async void HelpContents_OnClick(object? sender, RoutedEventArgs e)
    {
        await OpenUrlAsync(HelpContentsUrl, "Help contents could not be opened.");
    }

    private async void AboutLogicFriday_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new AboutDialog();
        await dialog.ShowDialog(this);
    }

    private void UnminimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowUnminimizedView();
        }
    }

    private void MinimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowMinimizedView();
        }
    }

    private void Minimize_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.MinimizeSelectedFunction();
            if (viewModel.GetSelectedFunction() is { } logicFunction)
            {
                ConfigureTruthTableColumns(FunctionTruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
            }
        }
    }

    private void GateZoomIn_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramSurface.ZoomIn();
        }
    }

    private void GateZoomOut_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramSurface.ZoomOut();
        }
    }

    private void GateZoomAll_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGateDiagramVisible: true })
        {
            return;
        }

        var contentBounds = GateDiagramSurface.ZoomAll(GateDiagramScrollViewer.Bounds.Size);
        GateDiagramScrollViewer.Offset = new Vector(
            Math.Max(0, contentBounds.Left * GateDiagramSurface.Zoom),
            Math.Max(0, contentBounds.Top * GateDiagramSurface.Zoom));
    }

    private void GateAutoRedraw_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true } viewModel)
        {
            var reroutedWireCount = GateDiagramSurface.AutoRedraw();
            viewModel.StatusText = reroutedWireCount == 0
                ? "Gate diagram redrawn"
                : $"Gate diagram redrawn: {reroutedWireCount} wire routes reset";
        }
    }

    private async void NewTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new TruthTableSetupDialog();
        var result = await dialog.ShowDialog<bool?>(this);
        if (result != true || DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.StartNewTruthTable(dialog.InputNames, dialog.OutputNames);
        ConfigureTruthTableColumns(TruthTableDataGrid, dialog.InputNames, dialog.OutputNames);
    }

    private void ModifyTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel ||
            !viewModel.StartModifyTruthTable() ||
            viewModel.GetSelectedFunction() is not { } logicFunction)
        {
            return;
        }

        ConfigureTruthTableColumns(TruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
    }

    private void NewLogicEquation_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.StartNewLogicEquation();
            EquationEditor.Focus();
        }
    }

    private void NewGateDiagram_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.StartNewGateDiagram();
        }
    }

    private async void ImportTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Truth Table",
            AllowMultiple = false,
            FileTypeFilter =
            [
                new FilePickerFileType("Truth Table Files")
                {
                    Patterns = [ "*.csv", "*.txt", "*.*" ]
                }
            ]
        });

        var file = files.FirstOrDefault();
        if (file is null)
        {
            return;
        }

        try
        {
            await using var stream = await file.OpenReadAsync();
            using var reader = new StreamReader(stream);
            var import = TruthTableImporter.Import(await reader.ReadToEndAsync());

            viewModel.StartImportedTruthTable(import.InputNames, import.OutputNames, import.OutputValues);
            ConfigureTruthTableColumns(TruthTableDataGrid, import.InputNames, import.OutputNames);
        }
        catch (TruthTableImportException ex)
        {
            await ShowMessageAsync($"The truth table could not be imported.\n{ex.Message}");
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"The truth table file could not be opened.\n{ex.Message}");
        }
    }

    private async void GatePaletteButton_OnClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: GatePaletteItem item } &&
            DataContext is MainWindowViewModel viewModel)
        {
            if (item.Kind == GatePaletteKind.Help)
            {
                await OpenUrlAsync(GateDiagramHelpUrl, "Gate diagram help could not be opened.");
                return;
            }

            if (item.Kind == GatePaletteKind.Cancel)
            {
                CancelGateDiagramEditing();
                return;
            }

            if (item.Kind == GatePaletteKind.Submit)
            {
                await SubmitGateDiagramEditingAsync();
                return;
            }

            SetActiveGatePaletteButton((Button)sender);
            viewModel.SelectGatePaletteItem(item);
        }
    }

    private void MainWindow_OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGateDiagramVisible: true })
        {
            return;
        }

        if (e.Key == Key.Enter && e.KeyModifiers == KeyModifiers.None)
        {
            _ = SubmitGateDiagramEditingAsync();
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Escape)
        {
            return;
        }

        CancelGateDiagramEditing();
        e.Handled = true;
    }

    private void GateDeleteSelected_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramSurface.DeleteSelected();
        }
    }

    private async void GateSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            await SubmitGateDiagramEditingAsync();
        }
    }

    private void GateCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            CancelGateDiagramEditing();
        }
    }

    private async Task SubmitGateDiagramEditingAsync()
    {
        ClearActiveGatePaletteButton();
        GateDiagramSurface.CancelInteraction();

        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        if (!viewModel.SubmitGateDiagramEditing(out var errorMessage))
        {
            if (!string.IsNullOrWhiteSpace(errorMessage))
            {
                await ShowMessageAsync(errorMessage, "Diagram Error");
            }

            return;
        }

        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            ConfigureTruthTableColumns(FunctionTruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void CancelGateDiagramEditing()
    {
        ClearActiveGatePaletteButton();
        GateDiagramSurface.CancelInteraction();

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelGateDiagramEditing();
        }
    }

    private void GateDiagramSurface_OnPaletteSelectionCleared(object? sender, EventArgs e)
    {
        ClearActiveGatePaletteButton();

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ClearGatePaletteSelection();
        }
    }

    private void SetActiveGatePaletteButton(Button button)
    {
        if (ReferenceEquals(_activeGatePaletteButton, button))
        {
            return;
        }

        ClearActiveGatePaletteButton();
        button.Classes.Add(ActiveGatePaletteButtonClass);
        _activeGatePaletteButton = button;
    }

    private void ClearActiveGatePaletteButton()
    {
        if (_activeGatePaletteButton is null)
        {
            return;
        }

        _activeGatePaletteButton.Classes.Remove(ActiveGatePaletteButtonClass);
        _activeGatePaletteButton = null;
    }

    private async void GateDiagramSurface_OnVariableNameRequested(
        object? sender,
        GateDiagramVariableNameRequestedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        var dialog = new GateVariableNameDialog
        {
            ExistingNames = viewModel.GateDiagramItems
                .Where(static item => item.Kind is GatePaletteKind.Input or GatePaletteKind.Output)
                .Select(static item => item.Label),
            VariableName = GetProposedGateVariableName(e.Item.Kind, viewModel.GateDiagramItems)
        };

        var result = await dialog.ShowDialog<bool?>(this);
        if (result == true)
        {
            e.AddItem(dialog.VariableName);
        }
    }

    private static string GetProposedGateVariableName(
        GatePaletteKind kind,
        IEnumerable<GateDiagramItem> items)
    {
        var existingNames = items
            .Where(static item => item.Kind is GatePaletteKind.Input or GatePaletteKind.Output)
            .Select(static item => item.Label)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        if (kind == GatePaletteKind.Output && !existingNames.Contains("X"))
        {
            return "X";
        }

        const string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        foreach (var name in alphabet.Select(static character => character.ToString()))
        {
            if (!existingNames.Contains(name))
            {
                return name;
            }
        }

        for (var index = 0; index < 100; index++)
        {
            var name = $"V{index}";
            if (!existingNames.Contains(name))
            {
                return name;
            }
        }

        return string.Empty;
    }

    private async Task OpenUrlAsync(string url, string errorMessage)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"{errorMessage}\n{ex.Message}");
        }
    }

    private void CloseDocument_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CloseCurrentDocument();
            TruthTableDataGrid.Columns.Clear();
            FunctionTruthTableDataGrid.Columns.Clear();
        }
    }

    private void Exit_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private void FunctionSummaryDataGrid_OnSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.SetSelectedFunctionCount(FunctionSummaryDataGrid.SelectedItems.Count);
        if (FunctionSummaryDataGrid.SelectedItems.Count != 1 ||
            viewModel.GetSelectedFunction() is not { } logicFunction)
        {
            return;
        }

        ConfigureTruthTableColumns(FunctionTruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
        viewModel.ShowFunction(logicFunction);
    }

    private void ConfigureTruthTableColumns(DataGrid dataGrid, string[] inputNames, string[] outputNames)
    {
        dataGrid.Columns.Clear();

        var headers = new[] { "Term" }
            .Concat(inputNames)
            .Concat([ "=>" ])
            .Concat(outputNames)
            .ToArray();

        var outputStartColumn = inputNames.Length + 2;
        for (var columnIndex = 0; columnIndex < headers.Length; columnIndex++)
        {
            dataGrid.Columns.Add(new DataGridTextColumn
            {
                Header = headers[columnIndex],
                Binding = new Binding($"Cells[{columnIndex}].Value"),
                IsReadOnly = true,
                Foreground = columnIndex >= outputStartColumn
                    ? FindThemeBrush("LogicFriday.Brush.Primary")
                    : FindThemeBrush("LogicFriday.Brush.OnSurface"),
                Width = columnIndex == 0 ? new DataGridLength(60) : DataGridLength.Auto
            });
        }
    }

    private static IBrush FindThemeBrush(string resourceKey)
    {
        if (Application.Current?.TryFindResource(resourceKey, out var resource) == true &&
            resource is IBrush brush)
        {
            return brush;
        }

        return Brushes.Black;
    }

    private async void EquationEditorContextMenu_OnOpening(object? sender, CancelEventArgs e)
    {
        var hasSelection = HasEquationEditorSelection();
        EquationEditorUndoMenuItem.IsEnabled = EquationEditor.CanUndo;
        EquationEditorRedoMenuItem.IsEnabled = EquationEditor.CanRedo;
        EquationEditorCutMenuItem.IsEnabled = hasSelection;
        EquationEditorCopyMenuItem.IsEnabled = hasSelection;
        EquationEditorPasteMenuItem.IsEnabled = false;
        EquationEditorDeleteMenuItem.IsEnabled = hasSelection;
        EquationEditorSelectAllMenuItem.IsEnabled = !string.IsNullOrEmpty(EquationEditor.Text);

        try
        {
            var clipboard = TopLevel.GetTopLevel(EquationEditor)?.Clipboard;
            if (clipboard is not null)
            {
                EquationEditorPasteMenuItem.IsEnabled = !string.IsNullOrEmpty(await clipboard.TryGetTextAsync());
            }
        }
        catch
        {
            EquationEditorPasteMenuItem.IsEnabled = false;
        }
    }

    private async void RefreshEquationEditorToolbarButtons()
    {
        var isEditingEquation = EquationEditor.IsVisible;
        var hasSelection = isEditingEquation && HasEquationEditorSelection();

        EquationEditorCutToolbarButton.IsEnabled = hasSelection;
        EquationEditorCopyToolbarButton.IsEnabled = hasSelection;
        EquationEditorUndoToolbarButton.IsEnabled = isEditingEquation && EquationEditor.CanUndo;
        EquationEditorRedoToolbarButton.IsEnabled = isEditingEquation && EquationEditor.CanRedo;
        EquationEditorPasteToolbarButton.IsEnabled = false;

        if (!isEditingEquation)
        {
            return;
        }

        try
        {
            var clipboard = TopLevel.GetTopLevel(EquationEditor)?.Clipboard;
            if (clipboard is not null)
            {
                EquationEditorPasteToolbarButton.IsEnabled = !string.IsNullOrEmpty(await clipboard.TryGetTextAsync());
            }
        }
        catch
        {
            EquationEditorPasteToolbarButton.IsEnabled = false;
        }
    }

    private void EquationEditorUndo_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Undo();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorRedo_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Redo();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorCut_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Cut();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorCopy_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Copy();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorPaste_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Paste();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorDelete_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.SelectedText = "";
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.SelectAll();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitLogicEquationEditing();
    }

    private void SubmitLogicEquationEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SubmitLogicEquationEditing();
            if (viewModel.GetSelectedFunction() is { } logicFunction)
            {
                ConfigureTruthTableColumns(FunctionTruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
            }
        }
    }

    private void EquationEditorCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelLogicEquationEditing();
        }
    }

    private bool HasEquationEditorSelection()
    {
        return EquationEditor.SelectionStart != EquationEditor.SelectionEnd;
    }

    private void EquationEditor_OnKeyDown(object? sender, KeyEventArgs e)
    {
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

        SubmitLogicEquationEditing();
        e.Handled = true;
    }

    private void EquationEditor_OnKeyUp(object? sender, KeyEventArgs e)
    {
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditor_OnPointerReleased(object? sender, PointerReleasedEventArgs e)
    {
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditor_OnTextChanged(object? sender, TextChangedEventArgs e)
    {
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditor_OnGotFocus(object? sender, RoutedEventArgs e)
    {
        RefreshEquationEditorToolbarButtons();
    }

    private void TruthTableDataGrid_OnCellPointerPressed(object? sender, DataGridCellPointerPressedEventArgs e)
    {
        if (e.PointerPressedEventArgs.GetCurrentPoint(TruthTableDataGrid).Properties.IsRightButtonPressed)
        {
            _truthTableContextRow = e.Row.DataContext as TruthTableRow;
        }

        if (e.PointerPressedEventArgs.ClickCount < 2 ||
            e.Row.DataContext is not TruthTableRow row)
        {
            return;
        }

        var columnIndex = TruthTableDataGrid.Columns.IndexOf(e.Column);
        if (columnIndex < 0 || columnIndex >= row.Cells.Count)
        {
            return;
        }

        row.Cells[columnIndex].CycleOutputValue();
    }

    private void TruthTableDataGrid_OnPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if (e.GetCurrentPoint(TruthTableDataGrid).Properties.IsRightButtonPressed)
        {
            _truthTableContextRow = null;
        }
    }

    private void TruthTableContextMenu_OnOpening(object? sender, CancelEventArgs e)
    {
        var hasDataRow = _truthTableContextRow is not null;
        TruthTableSetTrueMenuItem.IsEnabled = hasDataRow;
        TruthTableSetFalseMenuItem.IsEnabled = hasDataRow;
        TruthTableSetDontCareMenuItem.IsEnabled = hasDataRow;
        TruthTableInvertMenuItem.IsEnabled = hasDataRow;
    }

    private void TruthTableContextMenu_OnClosing(object? sender, CancelEventArgs e)
    {
        _truthTableContextRow = null;
    }

    private void TruthTableSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        TruthTableDataGrid.SelectedItems.Clear();
        foreach (var row in viewModel.TruthTableRows)
        {
            TruthTableDataGrid.SelectedItems.Add(row);
        }
    }

    private void TruthTableSetTrue_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowOutputValues("1");
    }

    private void TruthTableSetFalse_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowOutputValues("0");
    }

    private void TruthTableSetDontCare_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowOutputValues("X");
    }

    private void TruthTableInvert_OnClick(object? sender, RoutedEventArgs e)
    {
        foreach (var row in GetTruthTableContextRows())
        {
            foreach (var cell in row.Cells.Where(static cell => cell.IsOutput))
            {
                cell.Value = cell.Value switch
                {
                    "0" => "1",
                    "1" => "0",
                    _ => cell.Value
                };
            }
        }
    }

    private void TruthTableSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitTruthTableEditing();
    }

    private void TruthTableCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        CancelTruthTableEditing();
    }

    private void TruthTableDataGrid_OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
        {
            SubmitTruthTableEditing();
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Escape)
        {
            return;
        }

        CancelTruthTableEditing();
        e.Handled = true;
    }

    private void SubmitTruthTableEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SubmitTruthTableEditing();
            if (viewModel.GetSelectedFunction() is { } logicFunction)
            {
                ConfigureTruthTableColumns(FunctionTruthTableDataGrid, logicFunction.InputNames, logicFunction.OutputNames);
            }

            TruthTableDataGrid.Columns.Clear();
        }
    }

    private void CancelTruthTableEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelTruthTableEditing();
            TruthTableDataGrid.Columns.Clear();
        }
    }

    private void SetContextRowOutputValues(string value)
    {
        foreach (var row in GetTruthTableContextRows())
        {
            foreach (var cell in row.Cells.Where(static cell => cell.IsOutput))
            {
                cell.Value = value;
            }
        }
    }

    private IReadOnlyList<TruthTableRow> GetTruthTableContextRows()
    {
        var selectedRows = TruthTableDataGrid.SelectedItems
            .OfType<TruthTableRow>()
            .ToArray();

        if (selectedRows.Length > 0)
        {
            return selectedRows;
        }

        return _truthTableContextRow is null
            ? []
            : [ _truthTableContextRow ];
    }

    private async Task ShowMessageAsync(string message)
    {
        await ShowMessageAsync(message, "Logic Friday");
    }

    private async Task ShowMessageAsync(string message, string title)
    {
        var dialog = new Window
        {
            Title = title,
            Width = 380,
            Height = 150,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            CanResize = false
        };

        var okButton = new Button
        {
            Content = "OK",
            MinWidth = 80,
            HorizontalAlignment = HorizontalAlignment.Right,
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
                    TextWrapping = TextWrapping.Wrap
                },
                okButton
            }
        };

        Grid.SetRow(okButton, 1);
        await dialog.ShowDialog(this);
    }
}
