using System.Collections.ObjectModel;

namespace LogicFriday1.Models;

public sealed class TruthTableRow(IEnumerable<TruthTableCell> cells)
{
    public ObservableCollection<TruthTableCell> Cells { get; } = new(cells);
}
