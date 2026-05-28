using System.Buffers;

namespace Espresso;

public static class MinimumCoverSolver
{
    public static SparseEntry Solve(SparseMatrix A)
    {
        if (A.NRows <= 0) return new SparseEntry();
        var best = SolveRecursive(SparseMatrix.Clone(A), new CoverSolution(), 0, A.NCols + 1);
        var sol = new SparseEntry { Key = best!.Entry.Key, Refs = new SortedIntArray(best.Entry.Refs) };
        foreach (var prow in A.Rows)
            if (!prow.Refs.Overlaps(sol.Refs)) throw new InvalidOperationException("mincov: internal error -- cover verification failed\n");
        return sol;
    }
    private static CoverSolution? SolveRecursive(SparseMatrix A, CoverSolution select, int lb, int bound)
    {
        bool essenDone = false;
        {
            int delcols, delrows, essenCount;
            do
            {
                delcols = Dominance.ApplyDominance(A, false);
                var essen = new SortedIntArray();
                foreach (var prow in A.Rows)
                    if (prow.Refs.Count == 1) essen.Add(prow.Refs.Min);
                essenCount = essen.Count;
                int[] essenBuf = ArrayPool<int>.Shared.Rent(Math.Max(essenCount, 1));
                essen.CopyTo(essenBuf, 0);
                try
                {
                    for (int ei = 0; ei < essenCount; ei++)
                    {
                        select.AcceptColumn(A, essenBuf[ei]);
                        if (select.Cost >= bound) { essenDone = true; break; }
                    }
                }
                finally { ArrayPool<int>.Shared.Return(essenBuf); }
                if (essenDone) break;
                delrows = Dominance.ApplyDominance(A, true);
            } while (delcols > 0 || delrows > 0 || essenCount > 0);
        }
        if (select.Cost >= bound) return null;
        {
            SparseEntry c1g = default!, c2g = default!;
            int primaryRowNum = 0, secondaryRowNum = 0;
            bool reduceIt = false;
            foreach (var prow in A.Rows)
            {
                if (prow.Refs.Count != 2) continue;
                c1g = A.Cols[prow.Refs.Min]!;
                c2g = A.Cols[prow.Refs.Max]!;
                if (c1g.Refs.Count == 2) reduceIt = true;
                else if (c2g.Refs.Count == 2) { (c1g, c2g) = (c2g, c1g); reduceIt = true; }
                if (reduceIt)
                {
                    primaryRowNum = prow.Key;
                    secondaryRowNum = c1g.Refs.Min == primaryRowNum ? c1g.Refs.Max : c1g.Refs.Min;
                    break;
                }
            }
            if (reduceIt)
            {
                int c1Key = c1g.Key, c2Key = c2g.Key;
                var secEntry = A.Rows[secondaryRowNum]!;
                var saveSec = new SparseEntry { Key = secEntry.Key, Refs = new SortedIntArray(secEntry.Refs) };
                saveSec.Refs.Remove(c1Key);
                int c2RefCnt = c2g.Refs.Count;
                int[] c2RefBuf = ArrayPool<int>.Shared.Rent(c2RefCnt);
                c2g.Refs.CopyTo(c2RefBuf, 0);
                for (int ri = 0; ri < c2RefCnt; ri++)
                    if (c2RefBuf[ri] != primaryRowNum)
                        foreach (int col in saveSec.Refs) SparseMatrix.Insert(A, c2RefBuf[ri], col);
                ArrayPool<int>.Shared.Return(c2RefBuf);
                SparseMatrix.DeleteColumn(A, c1Key);
                SparseMatrix.DeleteColumn(A, c2Key);
                SparseMatrix.DeleteRow(A, primaryRowNum);
                SparseMatrix.DeleteRow(A, secondaryRowNum);
                var gBest = SolveRecursive(A, select, lb - 1, bound - 1);
                if (gBest != null)
                {
                    if (saveSec.Refs.Overlaps(gBest.Entry.Refs)) gBest.Add(c2Key);
                    else gBest.Add(c1Key);
                }
                return gBest;
            }
        }
        var indep = new CoverSolution();
        {
            var B = new SparseMatrix();
            foreach (var prow in A.Rows)
            {
                int totalCap = 0;
                foreach (int col in prow.Refs) totalCap += A.Cols[col]!.Refs.Count;
                int[] bimBuf = ArrayPool<int>.Shared.Rent(Math.Max(totalCap, 1));
                int bc = 0;
                foreach (int col in prow.Refs)
                {
                    var colEntry = A.Cols[col]!;
                    colEntry.Refs.CopyTo(bimBuf, bc);
                    bc += colEntry.Refs.Count;
                }
                Array.Sort(bimBuf, 0, bc);
                int unique = 0;
                for (int i = 0; i < bc; i++)
                    if (unique == 0 || bimBuf[unique - 1] != bimBuf[i]) bimBuf[unique++] = bimBuf[i];
                var rowSet = SortedIntArray.FromSorted(bimBuf.AsSpan(0, unique));
                ArrayPool<int>.Shared.Return(bimBuf);
                B.Rows.Set(prow.Key, new SparseEntry { Key = prow.Key, Refs = rowSet });
                foreach (int row in rowSet)
                {
                    if (!B.Cols.TryGetValue(row, out var pcol)) { pcol = new SparseEntry { Key = row }; B.Cols.Set(row, pcol); }
                    pcol.Refs.Add(prow.Key);
                }
            }
            while (B.Rows.Count > 0)
            {
                SparseEntry bestRow = null!;
                foreach (var bprow in B.Rows)
                    if (bestRow == null || bprow.Refs.Count < bestRow.Refs.Count) bestRow = bprow;
                indep.Cost++;
                indep.Entry.Refs.Add(bestRow.Key);
                int cnt = bestRow.Refs.Count;
                int[] buf = ArrayPool<int>.Shared.Rent(cnt);
                bestRow.Refs.CopyTo(buf, 0);
                for (int ci = 0; ci < cnt; ci++)
                {
                    SparseMatrix.DeleteRow(B, buf[ci]);
                    SparseMatrix.DeleteColumn(B, buf[ci]);
                }
                ArrayPool<int>.Shared.Return(buf);
            }
        }
        int lbNew = Math.Max(select.Cost + indep.Cost, lb);
        if (lbNew >= bound) return null;
        if (A.NRows == 0) return select.Clone();
        SparseMatrix tbpL = new(), tbpR = new();
        int blockResult = 0;
        if (A.NRows > 0)
        {
            SparseEntry? firstRow = null;
            foreach (var e in A.Rows) { firstRow = e; break; }
            var visitedRows = new HashSet<int>();
            var visitedCols = new HashSet<int>();
            if (!Dominance.Visit(A, firstRow!, visitedRows, visitedCols, A.Rows, A.Cols))
            {
                foreach (var prow in A.Rows)
                {
                    var target = visitedRows.Contains(prow.Key) ? tbpL : tbpR;
                    target.Rows.Set(prow.Key, new SparseEntry { Key = prow.Key, Refs = new SortedIntArray(prow.Refs) });
                    foreach (int col in prow.Refs)
                    {
                        if (!target.Cols.TryGetValue(col, out var pcol)) { pcol = new SparseEntry { Key = col }; target.Cols.Set(col, pcol); }
                        pcol.Refs.Add(prow.Key);
                    }
                }
                blockResult = 1;
            }
        }
        if (blockResult != 0)
        {
            if (tbpL.NCols > tbpR.NCols) { var t = tbpL; tbpL = tbpR; tbpR = t; }
            var leftBest = SolveRecursive(tbpL, new CoverSolution(), 0, bound - select.Cost);
            if (leftBest is null) return null;
            foreach (int col in leftBest.Entry.Refs) select.Add(col);
            return SolveRecursive(tbpR, select, lbNew, bound);
        }
        int branchCol;
        {
            int totalCap = 0;
            foreach (int indepRow in indep.Entry.Refs) totalCap += A.Rows[indepRow]!.Refs.Count;
            int[] sbBuf = ArrayPool<int>.Shared.Rent(Math.Max(totalCap, 1));
            int bc = 0;
            foreach (int indepRow in indep.Entry.Refs)
            {
                var rowEntry = A.Rows[indepRow]!;
                rowEntry.Refs.CopyTo(sbBuf, bc);
                bc += rowEntry.Refs.Count;
            }
            Array.Sort(sbBuf, 0, bc);
            int bestCol = -1; double bestW = -1.0;
            for (int i = 0; i < bc; i++)
            {
                if (i > 0 && sbBuf[i] == sbBuf[i - 1]) continue;
                var sbpcol = A.Cols[sbBuf[i]]!;
                double w = 0.0;
                foreach (int row in sbpcol.Refs) { var rEntry = A.Rows[row]!; w += 1.0 / ((double)rEntry.Refs.Count - 1.0); }
                if (w > bestW) { bestCol = sbpcol.Key; bestW = w; }
            }
            ArrayPool<int>.Shared.Return(sbBuf);
            branchCol = bestCol;
        }
        var A1 = SparseMatrix.Clone(A);
        var select1 = select.Clone();
        select1.AcceptColumn(A1, branchCol);
        return SolveRecursive(A1, select1, lbNew, bound);
    }
}