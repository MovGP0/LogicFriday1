using System.Buffers;

namespace Espresso;

public static class UnateComplement
{
    internal static BitVectorFamily ComplementRecursive(BitVectorFamily A)
    {
        if (A.Count == 0)
        {
            var r = BitVectorFamily.Create(1, A.SfSize);
            r.GetSpan(r.Count++).Clear();
            return r;
        }

        if (A.Count == 1)
        {
            ReadOnlySpan<uint> sp0 = A.GetSpan(0);
            var Abar = BitVectorFamily.Create(A.SfSize, A.SfSize);
            for (int i = 0; i < A.SfSize; i++)
            {
                if (BitVectorOps.Contains(sp0, i))
                {
                    Span<uint> sp1 = Abar.GetSpan(Abar.Count++);
                    sp1.Clear();
                    BitVectorOps.Insert(sp1, i);
                }
            }

            return Abar;
        }

        var prestrict = BitVectorOps.Create(A.SfSize);
        Span<uint> spr = prestrict.AsSpan();
        uint minSetOrd = (uint)(A.SfSize + 1);
        for (int si = 0; si < A.Count; si++)
        {
            uint sz = (uint)BitVectorOps.GetSortKey(A.GetSet(si));
            if (sz < minSetOrd)
            {
                BitVectorOps.Copy(spr, A.GetSpan(si));
                minSetOrd = sz;
            }
            else if (sz == minSetOrd)
            {
                BitVectorOps.Or(spr, spr, A.GetSpan(si));
            }
        }

        if (minSetOrd == 0)
        {
            A.Count = 0;
            return A;
        }

        if (minSetOrd == 1)
        {
            var rdf = BitVectorFamily.Create(A.Count, A.SfSize);
            for (int rsi = 0; rsi < A.Count; rsi++)
            {
                if (BitVectorOps.AreDisjoint(A.GetSpan(rsi), spr))
                {
                    Array.Copy(A.Data, rsi * A.Stride, rdf.Data, rdf.Count * rdf.Stride, A.Stride);
                    rdf.Count++;
                }
            }

            var Abar = ComplementRecursive(rdf);
            for (int si = 0; si < Abar.Count; si++)
            {
                BitVectorOps.Or(Abar.GetSpan(si), Abar.GetSpan(si), spr);
            }

            return Abar;
        }

        int maxI;
        {
            int words = A.Words;
            int[] sbcCount = ArrayPool<int>.Shared.Rent(A.SfSize);
            Array.Clear(sbcCount, 0, A.SfSize);
            for (int bsi = 0; bsi < A.Count; bsi++)
            {
                var bsp = A.GetSpan(bsi);
                int weight = 1024 / (BitVectorOps.PopCount(A.GetSpan(bsi)) - 1);
                for (int bi = 0; bi < words; bi++)
                {
                    uint bval = bsp[bi] & spr[bi];
                    int bb = bi << BitVectorOps.LogBpi;
                    while (bval != 0)
                    {
                        sbcCount[bb + System.Numerics.BitOperations.TrailingZeroCount(bval)] += weight;
                        bval &= bval - 1;
                    }
                }
            }

            int bestVar = -1, bestCount = 0;
            for (int bi = 0; bi < A.SfSize; bi++)
            {
                if (sbcCount[bi] > bestCount)
                {
                    bestVar = bi;
                    bestCount = sbcCount[bi];
                }
            }

            ArrayPool<int>.Shared.Return(sbcCount, clearArray: false);
            if (bestVar == -1)
            {
                throw new InvalidOperationException("abs_select_restricted: should not have best_var == -1");
            }

            maxI = bestVar;
        }

        var rncb = BitVectorFamily.Create(A.Count, A.SfSize);
        for (int rsi = 0; rsi < A.Count; rsi++)
        {
            if (!BitVectorOps.Contains(A.GetSpan(rsi), maxI))
            {
                Array.Copy(A.Data, rsi * A.Stride, rncb.Data, rncb.Count * rncb.Stride, A.Stride);
                rncb.Count++;
            }
        }

        var result = ComplementRecursive(rncb);
        for (int si = 0; si < result.Count; si++)
        {
            BitVectorOps.Insert(result.GetSpan(si), maxI);
        }

        for (int si = 0; si < A.Count; si++)
        {
            Span<uint> sp = A.GetSpan(si);
            if (BitVectorOps.Contains(sp, maxI))
            {
                BitVectorOps.Remove(sp, maxI);
                BitVectorOps.SetSortKey(A.GetSet(si), BitVectorOps.GetSortKey(A.GetSet(si)) - 1);
            }
        }

        return BitVectorFamily.Append(result, ComplementRecursive(A));
    }
}
