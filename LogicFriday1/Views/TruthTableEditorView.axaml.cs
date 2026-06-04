using System.ComponentModel;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Interactivity;
using LogicFriday1.Controls;
using LogicFriday1.Models;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class TruthTableEditorView : UserControl
{
    private TruthTableRow? _contextRow;

    public TruthTableEditorView()
    {
        InitializeComponent();
        TruthTableDataGrid.AddHandler(PointerPressedEvent, TruthTableDataGrid_OnPointerPressed, RoutingStrategies.Tunnel);
    }

    public event EventHandler? SelectionChanged;

    public event EventHandler? SubmitRequested;

    public event EventHandler? CancelRequested;

    public bool HasSelectedRows =>
        TruthTableDataGrid.SelectedItems
            .OfType<TruthTableRow>()
            .Any();

    public void ConfigureColumns(string[] inputNames, string[] outputNames)
    {
        TruthTableGridColumns.Configure(TruthTableDataGrid, inputNames, outputNames);
    }

    public void ClearColumns()
    {
        TruthTableDataGrid.Columns.Clear();
    }

    public void SelectAllEditableRows()
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        TruthTableDataGrid.Focus();
        TruthTableDataGrid.SelectedItems.Clear();
        foreach (var row in viewModel.TruthTableRows.Where(static row => row.Cells.Any(static cell => cell.IsOutput)))
        {
            TruthTableDataGrid.SelectedItems.Add(row);
        }
    }

    public void SetContextRowsOutputValues(string value)
    {
        foreach (var row in GetContextRows())
        {
            foreach (var cell in row.Cells.Where(static cell => cell.IsOutput))
            {
                cell.Value = value;
            }
        }
    }

    public void InvertContextRows()
    {
        foreach (var row in GetContextRows())
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

    private void TruthTableDataGrid_OnSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        SelectionChanged?.Invoke(this, EventArgs.Empty);
    }

    private void TruthTableDataGrid_OnCellPointerPressed(object? sender, DataGridCellPointerPressedEventArgs e)
    {
        if (e.PointerPressedEventArgs.GetCurrentPoint(TruthTableDataGrid).Properties.IsRightButtonPressed)
        {
            _contextRow = e.Row.DataContext as TruthTableRow;
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
            _contextRow = null;
        }
    }

    private void TruthTableContextMenu_OnOpening(object? sender, CancelEventArgs e)
    {
        var hasDataRow = _contextRow is not null;
        TruthTableSetTrueMenuItem.IsEnabled = hasDataRow;
        TruthTableSetFalseMenuItem.IsEnabled = hasDataRow;
        TruthTableSetDontCareMenuItem.IsEnabled = hasDataRow;
        TruthTableInvertMenuItem.IsEnabled = hasDataRow;
    }

    private void TruthTableContextMenu_OnClosing(object? sender, CancelEventArgs e)
    {
        _contextRow = null;
    }

    private void TruthTableSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        SelectAllEditableRows();
        SelectionChanged?.Invoke(this, EventArgs.Empty);
    }

    private void TruthTableSetTrue_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowsOutputValues("1");
    }

    private void TruthTableSetFalse_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowsOutputValues("0");
    }

    private void TruthTableSetDontCare_OnClick(object? sender, RoutedEventArgs e)
    {
        SetContextRowsOutputValues("X");
    }

    private void TruthTableInvert_OnClick(object? sender, RoutedEventArgs e)
    {
        InvertContextRows();
    }

    private void TruthTableSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitRequested?.Invoke(this, EventArgs.Empty);
    }

    private void TruthTableCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        CancelRequested?.Invoke(this, EventArgs.Empty);
    }

    private void TruthTableDataGrid_OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
        {
            SubmitRequested?.Invoke(this, EventArgs.Empty);
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Escape)
        {
            return;
        }

        CancelRequested?.Invoke(this, EventArgs.Empty);
        e.Handled = true;
    }

    private IReadOnlyList<TruthTableRow> GetContextRows()
    {
        var selectedRows = TruthTableDataGrid.SelectedItems
            .OfType<TruthTableRow>()
            .ToArray();

        if (selectedRows.Length > 0)
        {
            return selectedRows;
        }

        return _contextRow is null
            ? []
            : [_contextRow];
    }
}
