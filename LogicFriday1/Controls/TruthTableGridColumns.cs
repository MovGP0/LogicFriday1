using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;
using Avalonia.Media;

namespace LogicFriday1.Controls;

public static class TruthTableGridColumns
{
    public static void Configure(DataGrid dataGrid, string[] inputNames, string[] outputNames)
    {
        dataGrid.Columns.Clear();

        var headers = new[] { "Term" }
            .Concat(inputNames)
            .Concat(["=>"])
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
}
