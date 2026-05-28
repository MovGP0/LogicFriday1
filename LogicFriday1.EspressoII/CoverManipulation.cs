using System.Buffers;

namespace Espresso;

public static class CoverManipulation
{
    public static void CalculateCost(CubeData cube, BitVectorFamily F, out CoverCost cost)
    {
        cost = default;
        CubeList T = Cofactor.BuildCubeList(cube, F);
        var analysis = Cofactor.AnalyzeAllVariables(cube, T);
        T.ReturnCubes();
        cost.Cubes = F.Count;
        for (int var = 0; var < cube.NumBinaryVars; var++) cost.In += analysis.VarZeros[var];
        for (int var = cube.NumBinaryVars; var < cube.NumVars - 1; var++)
            cost.Mv += cube.IsSparse(var)
                ? F.Count * cube.PartSize[var] - analysis.VarZeros[var]
                : analysis.VarZeros[var];
        if (cube.NumBinaryVars != cube.NumVars)
            cost.Out = F.Count * cube.PartSize[cube.NumVars - 1] - analysis.VarZeros[cube.NumVars - 1];
        Cofactor.ReturnAnalysis(analysis);
        for (int si = 0; si < F.Count; si++)
            if (BitVectorOps.HasFlag(F.GetSet(si), CubeFlags.Prime)) cost.Primes++;
        cost.Total = cost.In + cost.Out + cost.Mv;
    }
    public static BitVectorFamily ExpandMultiValued(CubeData cube, BitVectorFamily B, int start) =>
        UnravelRange(cube, B, start, cube.NumVars - 1);
    public static BitVectorFamily SortByCoverage(CubeData cube, BitVectorFamily F, Comparison<BitVector> compare)
    {
        int n = cube.Size;
        // --- inlined ColumnCounts ---
        int[] count;
        {
            int ccWords = F.Words;
            count = ArrayPool<int>.Shared.Rent(F.SfSize);
            Array.Clear(count, 0, F.SfSize);
            for (int si = 0; si < F.Count; si++)
            {
                var cp = F.GetSpan(si);
                for (int ci = 0; ci < ccWords; ci++)
                {
                    uint val = cp[ci];
                    if (val != 0)
                    {
                        int b = ci << BitVectorOps.LogBpi;
                        while (val != 0)
                        {
                            count[b + System.Numerics.BitOperations.TrailingZeroCount(val)]++;
                            val &= val - 1;
                        }
                    }
                }
            }
        }
        // --- end inlined ColumnCounts ---
        int fWords = F.Words;
        for (int si = 0; si < F.Count; si++)
        {
            ReadOnlySpan<uint> sp = F.GetSpan(si);
            int cnt = 0;
            for (int ci = 0; ci < fWords; ci++)
            {
                uint val = sp[ci];
                int baseBit = ci << BitVectorOps.LogBpi;
                while (val != 0)
                {
                    cnt += count[baseBit + System.Numerics.BitOperations.TrailingZeroCount(val)];
                    val &= val - 1;
                }
            }
            BitVectorOps.SetSortKey(F.GetSet(si), cnt);
        }
        ArrayPool<int>.Shared.Return(count, clearArray: false);
        int[] order = ArrayPool<int>.Shared.Rent(F.Count);
        for (int i = 0; i < F.Count; i++) order[i] = i;
        order.AsSpan(0, F.Count).Sort((a, b) => compare(F.GetSet(a), F.GetSet(b)));
        var result = BitVectorFamily.FromSortedOrder(F, order, F.Count);
        ArrayPool<int>.Shared.Return(order, clearArray: false);
        return result;
    }
    public static int PartitionCubeList(CubeData cube, CubeList T, out CubeList A, out CubeList B)
    {
        int n = T.Count;
        bool[] covered = ArrayPool<bool>.Shared.Rent(n);
        covered.AsSpan(0, n).Clear();
        BitVector seed = cube.RentCof();
        T[0].AsSpan().CopyTo(seed.AsSpan());
        covered[0] = true;
        int count = 1;
        Span<uint> seedSpan = seed.AsSpan();
        ReadOnlySpan<uint> cofSpan = T.CofSpan;
        bool change;
        do
        {
            change = false;
            for (int i = 0; i < n; i++)
            {
                if (covered[i]) continue;
                ReadOnlySpan<uint> sp = T.GetSpan(i);
                // Inline HaveCommonActive
                {
                    int hcLast = cube.InWord;
                    if (hcLast != -1)
                    {
                        uint x = sp[hcLast] | cofSpan[hcLast], y = seedSpan[hcLast] | cofSpan[hcLast];
                        if ((~(x & (x >> 1)) & ~(y & (y >> 1)) & cube.InMask) != 0) goto haveCommon;
                        for (int w = 0; w < hcLast; w++)
                        {
                            x = sp[w] | cofSpan[w]; y = seedSpan[w] | cofSpan[w];
                            if ((~(x & (x >> 1)) & ~(y & (y >> 1)) & BitVectorOps.Disjoint) != 0) goto haveCommon;
                        }
                    }
                    for (int var2 = cube.NumBinaryVars; var2 < cube.NumVars; var2++)
                    {
                        ReadOnlySpan<uint> sm = cube.VarMask[var2].AsSpan();
                        int mvLast = cube.LastWordOf(var2);
                        for (int w = cube.FirstWordOf(var2); w <= mvLast; w++)
                            if ((sm[w] & ~sp[w] & ~cofSpan[w]) != 0)
                            {
                                for (int w2 = cube.FirstWordOf(var2); w2 <= mvLast; w2++)
                                    if ((sm[w2] & ~seedSpan[w2] & ~cofSpan[w2]) != 0) goto haveCommon;
                                break;
                            }
                    }
                    continue;
                }
                haveCommon:
                BitVectorOps.And(seedSpan, seedSpan, sp);
                covered[i] = true;
                change = true;
                count++;
            }
        } while (change);
        if (count != n)
        {
            var aCubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(count, 1));
            var bCubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(n - count, 1));
            int ai = 0, bi = 0;
            for (int i = 0; i < n; i++)
            {
                if (covered[i]) aCubes[ai++] = T[i];
                else bCubes[bi++] = T[i];
            }
            BitVector cofA = cube.RentCof();
            BitVector cofB = cube.RentCof();
            T.Cof.AsSpan().CopyTo(cofA.AsSpan());
            T.Cof.AsSpan().CopyTo(cofB.AsSpan());
            A = new CubeList(cofA, aCubes, ai, rented: true, ownsCof: true, owner: cube);
            B = new CubeList(cofB, bCubes, bi, rented: true, ownsCof: true, owner: cube);
        }
        else { A = new(BitVector.Null, [], 0); B = new(BitVector.Null, [], 0); }
        cube.ReturnCof(seed);
        ArrayPool<bool>.Shared.Return(covered);
        return T.Count - count;
    }
    private static BitVectorFamily UnravelRange(CubeData cube, BitVectorFamily B, int start, int end)
    {
        Span<uint> sbSpan = cube.Temp![1].AsSpan();
        BitVectorOps.Copy(sbSpan, cube.EmptySet.AsSpan());
        for (int var = 0; var < start; var++) BitVectorOps.Or(sbSpan, sbSpan, cube.VarMask![var].AsSpan());
        for (int var = end + 1; var < cube.NumVars; var++) BitVectorOps.Or(sbSpan, sbSpan, cube.VarMask![var].AsSpan());
        int totalSize = 0;
        for (int si = 0; si < B.Count; si++)
        {
            ReadOnlySpan<uint> sp = B.GetSpan(si);
            int expansion = 1;
            for (int var = start; var <= end; var++)
            {
                int size = BitVectorOps.IntersectionCount(sp, cube.VarMask![var].AsSpan());
                if (size >= 2)
                {
                    expansion *= size;
                    if (expansion > 1000000) throw new InvalidOperationException("unreasonable expansion in unravel");
                }
            }
            totalSize += expansion;
        }
        if (totalSize == B.Count) return B;  // No expansion needed
        var B1 = BitVectorFamily.Create(totalSize, cube.Size);
        for (int si = 0; si < B.Count; si++)
        {
            ReadOnlySpan<uint> c = B.GetSpan(si);
            Span<uint> baseSpan = cube.Temp![0].AsSpan();
            int expansion = 1, place, skip, size;
            BitVectorOps.Copy(baseSpan, sbSpan);
            for (int var = start; var <= end; var++)
            {
                ReadOnlySpan<uint> vm = cube.VarMask![var].AsSpan();
                if ((size = BitVectorOps.IntersectionCount(c, vm)) < 2) BitVectorOps.Or(baseSpan, baseSpan, vm);
                else expansion *= size;
            }
            BitVectorOps.And(baseSpan, c, baseSpan);
            int offset = B1.Count;
            B1.Count += expansion;
            for (int pi = offset; pi < B1.Count; pi++) BitVectorOps.Copy(B1.GetSpan(pi), baseSpan);
            place = expansion;
            for (int var = start; var <= end; var++)
            {
                ReadOnlySpan<uint> vm = cube.VarMask![var].AsSpan();
                if ((size = BitVectorOps.IntersectionCount(c, vm)) <= 1) continue;
                skip = place;
                place /= size;
                int n = 0;
                for (int i = cube.FirstPart![var]; i <= cube.LastPart![var]; i++)
                {
                    if (!BitVectorOps.Contains(c, i)) continue;
                    for (int j = n; j < expansion; j += skip)
                    for (int k = 0; k < place; k++) BitVectorOps.Insert(B1.GetSpan(j + k + offset), i);
                    n += place;
                }
            }
        }
        return B1;
    }
}