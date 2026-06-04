using Avalonia.Controls;
using LogicFriday1.Controls;

namespace LogicFriday1.Views;

public partial class FunctionTruthTableView : UserControl
{
    public FunctionTruthTableView()
    {
        InitializeComponent();
    }

    public void ConfigureColumns(string[] inputNames, string[] outputNames)
    {
        TruthTableGridColumns.Configure(TruthTableDataGrid, inputNames, outputNames);
    }

    public void ClearColumns()
    {
        TruthTableDataGrid.Columns.Clear();
    }
}
