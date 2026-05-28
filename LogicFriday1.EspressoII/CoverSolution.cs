using System.Buffers;

namespace Espresso;

public class CoverSolution
{
    public SparseEntry Entry = new();

    public int Cost;

    public CoverSolution Clone() => new()
    {
        Cost = Cost,
        Entry = new SparseEntry
        {
            Key = Entry.Key,
            Refs = new SortedIntArray(Entry.Refs)
        }
    };

    public void Add(int col)
    {
        Entry.Refs.Add(col);
        Cost++;
    }

    public void AcceptColumn(SparseMatrix A, int col)
    {
        Add(col);
        if (!A.Cols.TryGetValue(col, out var pcol))
        {
            return;
        }

        int rowCnt = pcol.Refs.Count;
        int[] rowBuf = ArrayPool<int>.Shared.Rent(rowCnt);
        pcol.Refs.CopyTo(rowBuf, 0);
        for (int ri = 0; ri < rowCnt; ri++)
        {
            SparseMatrix.DeleteRow(A, rowBuf[ri]);
        }

        ArrayPool<int>.Shared.Return(rowBuf);
    }
}
