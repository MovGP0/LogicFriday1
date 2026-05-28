using System.Buffers;

namespace Espresso;

public static class Dominance
{
    public static int ApplyDominance(SparseMatrix A, bool isRow)
    {
        var primary = isRow ? A.Rows : A.Cols;
        var secondary = isRow ? A.Cols : A.Rows;
        int initCount = primary.Count;
        int[] keyBuf = ArrayPool<int>.Shared.Rent(Math.Max(initCount, 1));
        primary.CopyKeysTo(keyBuf);
        for (int ki = 0; ki < initCount; ki++)
        {
            if (!primary.TryGetValue(keyBuf[ki], out var entry)) continue;
            if (entry.Refs.Count == 0) continue;
            var least = secondary[entry.Refs.Min]!;
            foreach (int r in entry.Refs)
            {
                var s = secondary[r]!;
                if (s.Refs.Count < least.Refs.Count) least = s;
            }
            int leastCnt = least.Refs.Count;
            int[] leastBuf = ArrayPool<int>.Shared.Rent(leastCnt);
            least.Refs.CopyTo(leastBuf, 0);
            if (isRow)
            {
                for (int ri = 0; ri < leastCnt; ri++)
                {
                    var other = primary[leastBuf[ri]];
                    if (other == null) continue;
                    if (other.Refs.Count > entry.Refs.Count ||
                        (other.Refs.Count == entry.Refs.Count && other.Key > entry.Key))
                        if (other.Refs.IsSupersetOf(entry.Refs))
                            SparseMatrix.Delete(primary, secondary, other.Key);
                }
            }
            else
            {
                bool deleted = false;
                for (int ci = 0; ci < leastCnt && !deleted; ci++)
                {
                    var other = primary[leastBuf[ci]];
                    if (other == null) continue;
                    if (other.Refs.Count > entry.Refs.Count ||
                        (other.Refs.Count == entry.Refs.Count && other.Key > entry.Key))
                        if (other.Refs.IsSupersetOf(entry.Refs))
                        {
                            SparseMatrix.Delete(primary, secondary, entry.Key);
                            deleted = true;
                        }
                }
            }
            ArrayPool<int>.Shared.Return(leastBuf);
        }
        ArrayPool<int>.Shared.Return(keyBuf);
        return initCount - primary.Count;
    }
    internal static bool Visit(SparseMatrix A, SparseEntry entry, HashSet<int> visitedSelf, HashSet<int> visitedOther,
        SparseDict selfDict, SparseDict otherDict)
    {
        if (!visitedSelf.Add(entry.Key)) return false;
        if (visitedSelf.Count == selfDict.Count) return true;
        foreach (int r in entry.Refs)
            if (Visit(A, otherDict[r]!, visitedOther, visitedSelf, otherDict, selfDict)) return true;
        return false;
    }
}