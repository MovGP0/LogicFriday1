using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
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
using LogicFriday1.Models;
using LogicFriday1.Services;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MainWindow : Window
{
    private const string HelpFileName = "lf.chm";
    private const string HelpContentsTopic = "features.htm";
    private TruthTableRow? _truthTableContextRow;

    public MainWindow()
    {
        InitializeComponent();
        TruthTableDataGrid.AddHandler(PointerPressedEvent, TruthTableDataGrid_OnPointerPressed, RoutingStrategies.Tunnel);
    }

    private async void HelpContents_OnClick(object? sender, RoutedEventArgs e)
    {
        if (!OperatingSystem.IsWindows())
        {
            await ShowMessageAsync("Help contents require Windows HTML Help.");
            return;
        }

        var helpFilePath = FindHelpFilePath();
        if (helpFilePath is null)
        {
            await ShowMessageAsync("Help contents are not available because lf.chm was not found.");
            return;
        }

        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = "hh.exe",
                Arguments = $"\"{helpFilePath}::/{HelpContentsTopic}\"",
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"Help contents could not be opened.\n{ex.Message}");
        }
    }

    private async void AboutLogicFriday_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new AboutDialog();
        await dialog.ShowDialog(this);
    }

    private void MinimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowMinimizedView();
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

    private void GatePaletteButton_OnClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: GatePaletteItem item } &&
            DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SelectGatePaletteItem(item);
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
        if (DataContext is not MainWindowViewModel viewModel ||
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

    private void EquationEditorUndo_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Undo();
    }

    private void EquationEditorRedo_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Redo();
    }

    private void EquationEditorCut_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Cut();
    }

    private void EquationEditorCopy_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Copy();
    }

    private void EquationEditorPaste_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.Paste();
    }

    private void EquationEditorDelete_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.SelectedText = "";
    }

    private void EquationEditorSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        EquationEditor.Focus();
        EquationEditor.SelectAll();
    }

    private void EquationEditorSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SubmitLogicEquationEditing();
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

    private static string? FindHelpFilePath()
    {
        var candidateDirectories = new[]
        {
            AppContext.BaseDirectory,
            Environment.CurrentDirectory,
            Path.Combine(AppContext.BaseDirectory, "Help")
        };

        foreach (var directory in candidateDirectories)
        {
            var helpFilePath = Path.Combine(directory, HelpFileName);
            if (File.Exists(helpFilePath))
            {
                return helpFilePath;
            }
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
