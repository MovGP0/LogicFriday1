using Avalonia.Controls;
using LogicFriday1.Models;

namespace LogicFriday1.Views;

public partial class FunctionsGridView : UserControl
{
    public FunctionsGridView()
    {
        InitializeComponent();
    }

    public event EventHandler? SelectionChanged;

    public IReadOnlyList<FunctionSummaryRow> SelectedSummaries =>
        FunctionSummaryDataGrid.SelectedItems
            .OfType<FunctionSummaryRow>()
            .ToArray();

    public int SelectedItemCount => FunctionSummaryDataGrid.SelectedItems.Count;

    private void FunctionSummaryDataGrid_OnSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        SelectionChanged?.Invoke(this, EventArgs.Empty);
    }
}
