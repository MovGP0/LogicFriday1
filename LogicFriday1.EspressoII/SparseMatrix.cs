namespace Espresso;

public class SparseMatrix(int rowCap = 16, int colCap = 16)
{
    public SparseDict Rows = new(rowCap), Cols = new(colCap);
    public int LastRowNum = -1;
    public int NRows => Rows.Count;
    public int NCols => Cols.Count;

    public static SparseMatrix Clone(SparseMatrix A)
    {
        var B = new SparseMatrix(A.Rows.Capacity, A.Cols.Capacity);
        foreach (var e in A.Rows) B.Rows.Set(e.Key, new SparseEntry { Key = e.Key, Refs = new SortedIntArray(e.Refs) });
        foreach (var e in A.Cols) B.Cols.Set(e.Key, new SparseEntry { Key = e.Key, Refs = new SortedIntArray(e.Refs) });
        B.LastRowNum = A.LastRowNum;
        return B;
    }
    public static void Insert(SparseMatrix A, int row, int col)
    {
        if (!A.Rows.TryGetValue(row, out var prow)) { prow = new SparseEntry { Key = row }; A.Rows.Set(row, prow); if (row > A.LastRowNum) A.LastRowNum = row; }
        if (!A.Cols.TryGetValue(col, out var pcol)) { pcol = new SparseEntry { Key = col }; A.Cols.Set(col, pcol); }
        prow.Refs.Add(col);
        pcol.Refs.Add(row);
    }
    public static void Delete(SparseDict primary, SparseDict secondary, int i)
    {
        if (!primary.TryGetValue(i, out var entry)) return;
        foreach (int r in entry.Refs)
            if (secondary.TryGetValue(r, out var other))
            {
                other.Refs.Remove(i);
                if (other.Refs.Count == 0) secondary.Remove(r);
            }
        primary.Remove(i);
    }
    public static void DeleteRow(SparseMatrix A, int i) => Delete(A.Rows, A.Cols, i);
    public static void DeleteColumn(SparseMatrix A, int i) => Delete(A.Cols, A.Rows, i);
}