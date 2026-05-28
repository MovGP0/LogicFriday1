using System.Buffers;

namespace Espresso;

public static class Expander
{
    public static BitVectorFamily ExpandCover(CubeData cube, BitVectorFamily F, BitVectorFamily R, int nonsparse)
    {
        MemoCache.Key key = default;
        bool cacheActive = MemoCache.Enabled;
        if (cacheActive)
        {
            key = MemoCache.BuildFamiliesKey(MemoCache.TagExpandCover, cube, F, R, null, nonsparse);
            if (MemoCache.TryGetFamily(key, cube.Size, out var cached))
            {
                return cached;
            }
        }

        var result = ExpandCoverImpl(cube, F, R, nonsparse);
        if (cacheActive)
        {
            MemoCache.PutFamily(key, result);
        }

        return result;
    }

    private static BitVectorFamily ExpandCoverImpl(CubeData cube, BitVectorFamily F, BitVectorFamily R, int nonsparse)
    {
        int[] countBuf = ArrayPool<int>.Shared.Rent(cube.Size);
        F = CoverManipulation.SortByCoverage(cube, F, BitVectorOps.CompareAscending);
        // --- inlined CreateBatch ---
        BitVector[] scratch;
        {
            int cbWords = BitVectorOps.WordCount(cube.Size);
            int cbStride = cbWords + 1;
            var cbData = new uint[5 * cbStride];
            scratch = new BitVector[5];
            for (int cbi = 0; cbi < 5; cbi++)
            {
                scratch[cbi] = new BitVector(cbData, cbi * cbStride + 1, cbWords);
            }
        }

        // --- end inlined CreateBatch ---
        BitVector RAISE = scratch[0], FREESET = scratch[1], INIT_LOWER = scratch[2],
            SUPER_CUBE = scratch[3], OVEREXPANDED_CUBE = scratch[4];
        Span<uint> sRAISE = RAISE.AsSpan(), sFREESET = FREESET.AsSpan(), sINIT_LOWER = INIT_LOWER.AsSpan(),
            sSUPER_CUBE = SUPER_CUBE.AsSpan(), sOVEREXPANDED_CUBE = OVEREXPANDED_CUBE.AsSpan();
        if (nonsparse != 0)
        {
            for (int var = 0; var < cube.NumVars; var++)
            {
                if (cube.IsSparse(var))
                {
                    BitVectorOps.Or(sINIT_LOWER, sINIT_LOWER, cube.VarMask![var].AsSpan());
                }
            }
        }

        BitVectorFamily.ClearAllFlags(F, CubeFlags.Covered | CubeFlags.NonEssen);
        var newLowerBuf = BitVectorFamily.Create(Math.Max(F.Count, 16), cube.Size);
        int[] feasIdxBuf = ArrayPool<int>.Shared.Rent(Math.Max(256, F.Count));
        for (int i = 0; i < F.Count; i++)
        {
            BitVector p = F.GetSet(i);
            if (!BitVectorOps.HasFlag(p, CubeFlags.Prime) && !BitVectorOps.HasFlag(p, CubeFlags.Covered))
            {
                // --- inlined ExpandOneCube ---
                BitVectorFamily BB = R, CC = F;
                ReadOnlySpan<uint> sFull = cube.FullSet.AsSpan();
                BitVectorOps.AddFlag(p, CubeFlags.Prime);
                BitVectorFamily.ActivateAll(BB);
                BitVectorFamily.ActivateAll(CC);
                for (int ei = 0; ei < CC.Count; ei++)
                {
                    BitVector sbcp = CC.GetSet(ei);
                    if (BitVectorOps.HasFlag(sbcp, CubeFlags.Covered) || BitVectorOps.HasFlag(sbcp, CubeFlags.Prime))
                    {
                        CC.ActiveCount--;
                        BitVectorOps.ClearFlag(sbcp, CubeFlags.Active);
                    }
                }

                int num_covered = 0;
                ReadOnlySpan<uint> sc = p.AsSpan();
                BitVectorOps.Copy(sSUPER_CUBE, sc);
                BitVectorOps.Copy(sRAISE, sc);
                BitVectorOps.AndNot(sFREESET, sFull, sRAISE);
                if (!BitVectorOps.IsEmpty(sINIT_LOWER))
                {
                    BitVectorOps.AndNot(sFREESET, sFREESET, sINIT_LOWER);
                    EliminateLowering(cube, BB, CC, RAISE, FREESET);
                }

                DetermineEssentialParts(cube, BB, CC, RAISE, FREESET);
                BitVectorOps.Or(sOVEREXPANDED_CUBE, sRAISE, sFREESET);
                if (CC.ActiveCount > 0)
                {
                    ReadOnlySpan<uint> sEmpty = cube.EmptySet.AsSpan();
                    Span<int> feasIdx = feasIdxBuf;
                    int numfeas = 0;
                    for (int ii = 0; ii < CC.Count; ii++)
                    {
                        if (BitVectorOps.HasFlag(CC.GetSet(ii), CubeFlags.Active))
                        {
                            feasIdx[numfeas++] = ii;
                        }
                    }

                    BitVectorFamily new_lower = numfeas <= newLowerBuf.Capacity ? newLowerBuf : BitVectorFamily.Create(numfeas, cube.Size);
                    if (numfeas <= newLowerBuf.Capacity)
                    {
                        new_lower.Count = 0;
                    }

                    while (true)
                    {
                        Span<uint> sxraise = cube.Temp![0].AsSpan();
                        BitVectorOps.Copy(sxraise, sEmpty);
                        for (int j = 0; j < BB.Count; j++)
                        {
                            if (BitVectorOps.HasFlag(BB.GetSet(j), CubeFlags.Active))
                            {
                                BitVectorOps.Or(sxraise, sxraise, BB.GetSpan(j));
                            }
                        }

                        BitVectorOps.AndNot(sxraise, sFREESET, sxraise);
                        BitVectorOps.Or(sRAISE, sRAISE, sxraise);
                        BitVectorOps.AndNot(sFREESET, sFREESET, sxraise);
                        int lastfeas = numfeas;
                        numfeas = 0;
                        for (int fi = 0; fi < lastfeas; fi++)
                        {
                            BitVector fp = CC.GetSet(feasIdx[fi]);
                            if (BitVectorOps.HasFlag(fp, CubeFlags.Active))
                            {
                                ReadOnlySpan<uint> sp = CC.GetSpan(feasIdx[fi]);
                                if (BitVectorOps.IsSubsetOf(sp, sRAISE))
                                {
                                    num_covered++;
                                    BitVectorOps.Or(sSUPER_CUBE, sSUPER_CUBE, sp);
                                    CC.ActiveCount--;
                                    BitVectorOps.ClearFlag(fp, CubeFlags.Active);
                                    BitVectorOps.AddFlag(fp, CubeFlags.Covered);
                                }
                                else
                                {
                                    int ifcResult;
                                    {
                                        Span<uint> sifcr = cube.Temp![0].AsSpan();
                                        BitVectorOps.Or(sifcr, RAISE.AsSpan(), fp.AsSpan());
                                        BitVectorOps.Copy(new_lower.GetSet(numfeas).AsSpan(), cube.EmptySet.AsSpan());
                                        ifcResult = 1;
                                        for (int bi = 0; bi < BB.Count; bi++)
                                        {
                                            if (!BitVectorOps.HasFlag(BB.GetSet(bi), CubeFlags.Active))
                                            {
                                                continue;
                                            }

                                            int dist = CubeDistance.DistanceCapped(cube, BB.GetSpan(bi), sifcr);
                                            if (dist > 1)
                                            {
                                                continue;
                                            }

                                            if (dist == 0)
                                            {
                                                ifcResult = 0;
                                                break;
                                            }

                                            CubeDistance.FindDisjointParts(cube, new_lower.GetSet(numfeas).AsSpan(), BB.GetSpan(bi), sifcr);
                                        }
                                    }

                                    if (ifcResult != 0)
                                    {
                                        feasIdx[numfeas] = feasIdx[fi];
                                        numfeas++;
                                    }
                                }
                            }
                        }

                        if (numfeas == 0)
                        {
                            break;
                        }

                        int bestcount = 0, bestsize = 9999, bestFeasIdx = -1;
                        for (int fi = 0; fi < numfeas; fi++)
                        {
                            int size = BitVectorOps.IntersectionCount(CC.GetSpan(feasIdx[fi]), sFREESET);
                            int count = 0;
                            for (int fj = 0; fj < numfeas; fj++)
                            {
                                if (BitVectorOps.AreDisjoint(new_lower.GetSpan(fi), CC.GetSpan(feasIdx[fj])))
                                {
                                    count++;
                                }
                            }

                            if (count > bestcount)
                            {
                                bestcount = count;
                                bestFeasIdx = feasIdx[fi];
                                bestsize = size;
                            }
                            else if (count == bestcount && size < bestsize)
                            {
                                bestFeasIdx = feasIdx[fi];
                                bestsize = size;
                            }
                        }

                        BitVectorOps.Or(sRAISE, sRAISE, CC.GetSpan(bestFeasIdx));
                        BitVectorOps.AndNot(sFREESET, sFREESET, sRAISE);
                        DetermineEssentialParts(cube, BB, CC, RAISE, FREESET);
                    }
                }

                while (CC.ActiveCount > 0)
                {
                    int ebi = SelectMostFrequent(cube, CC, FREESET, countBuf);
                    BitVectorOps.Insert(sRAISE, ebi);
                    BitVectorOps.Remove(sFREESET, ebi);
                    DetermineEssentialParts(cube, BB, CC, RAISE, FREESET);
                }

                while (BB.ActiveCount > 0)
                {
                    Span<uint> mcxraise = cube.Temp![0].AsSpan();
                    ReadOnlySpan<uint> mcEmpty = cube.EmptySet.AsSpan();
                    var B = BitVectorFamily.Create(BB.ActiveCount, cube.Size);
                    for (int mi = 0; mi < BB.Count; mi++)
                    {
                        if (BitVectorOps.HasFlag(BB.GetSet(mi), CubeFlags.Active))
                        {
                            Span<uint> splower = B.GetSpan(B.Count++);
                            BitVectorOps.Copy(splower, mcEmpty);
                            CubeDistance.FindDisjointParts(cube, splower, BB.GetSpan(mi), sRAISE);
                        }
                    }

                    int nset = 0;
                    bool useHeuristic = false;
                    for (int mi = 0; mi < B.Count; mi++)
                    {
                        Span<uint> bsp = B.GetSpan(mi);
                        int expansion = 1;
                        for (int v = cube.NumBinaryVars; v < cube.NumVars; v++)
                        {
                            int edist = BitVectorOps.IntersectionCount(bsp, cube.VarMask![v].AsSpan());
                            if (edist > 1)
                            {
                                expansion *= edist;
                                if (expansion > 500)
                                {
                                    useHeuristic = true;
                                    break;
                                }
                            }
                        }

                        if (useHeuristic)
                        {
                            break;
                        }

                        nset += expansion;
                        if (nset > 500)
                        {
                            useHeuristic = true;
                            break;
                        }
                    }

                    if (!useHeuristic)
                    {
                        B = CoverManipulation.ExpandMultiValued(cube, B, cube.NumBinaryVars);
                        // --- inlined SolveFromFamily ---
                        BitVector xlower;
                        {
                            var sfM = new SparseMatrix();
                            for (int _pi = 0; _pi < B.Count; _pi++)
                            {
                                var sfsp = B.GetSpan(_pi);
                                for (int sfi = sfsp.Length - 1; sfi >= 0; sfi--)
                                {
                                    uint sfval = sfsp[sfi];
                                    int sfBase = sfi << BitVectorOps.LogBpi;
                                    while (sfval != 0)
                                    {
                                        SparseMatrix.Insert(sfM, _pi, sfBase + System.Numerics.BitOperations.TrailingZeroCount(sfval));
                                        sfval &= sfval - 1;
                                    }
                                }
                            }

                            var sfCover = MinimumCoverSolver.Solve(sfM);
                            xlower = BitVectorOps.Create(B.SfSize);
                            Span<uint> sfsc = xlower.AsSpan();
                            foreach (int col in sfCover.Refs)
                            {
                                BitVectorOps.Insert(sfsc, col);
                            }
                        }

                        // --- end inlined SolveFromFamily ---
                        BitVectorOps.AndNot(mcxraise, sFREESET, xlower.AsSpan());
                        BitVectorOps.Or(sRAISE, sRAISE, mcxraise);
                        BitVectorOps.Copy(sFREESET, mcEmpty);
                        BB.ActiveCount = 0;
                    }
                    else
                    {
                        BitVectorOps.Insert(sRAISE, SelectMostFrequent(cube, null, FREESET, countBuf));
                        BitVectorOps.AndNot(sFREESET, sFREESET, sRAISE);
                        DetermineEssentialParts(cube, BB, null, RAISE, FREESET);
                    }
                }

                BitVectorOps.Or(sRAISE, sRAISE, sFREESET);
                // --- end inlined ExpandOneCube ---
                BitVectorOps.Copy(F.GetSpan(i), sRAISE);
                BitVectorOps.AddFlag(p, CubeFlags.Prime);
                if (num_covered == 0 && !BitVectorOps.AreEqual(F.GetSpan(i), sOVEREXPANDED_CUBE))
                {
                    BitVectorOps.AddFlag(p, CubeFlags.NonEssen);
                }
            }
        }

        F.ActiveCount = 0;
        bool change = false;
        for (int i = 0; i < F.Count; i++)
        {
            BitVector p = F.GetSet(i);
            if (BitVectorOps.HasFlag(p, CubeFlags.Covered))
            {
                BitVectorOps.ClearFlag(p, CubeFlags.Active);
                change = true;
            }
            else
            {
                BitVectorOps.AddFlag(p, CubeFlags.Active);
                F.ActiveCount++;
            }
        }

        if (change)
        {
            F = BitVectorFamily.CompactInactive(F);
        }

        ArrayPool<int>.Shared.Return(feasIdxBuf, clearArray: false);
        ArrayPool<int>.Shared.Return(countBuf, clearArray: false);
        return F;
    }

    internal static void DetermineEssentialParts(CubeData cube, BitVectorFamily BB, BitVectorFamily? CC, BitVector RAISE, BitVector FREESET)
    {
        Span<uint> sRAISE = RAISE.AsSpan(), sFREESET = FREESET.AsSpan();
        Span<uint> sxlower = cube.Temp![0].AsSpan();
        BitVectorOps.Copy(sxlower, cube.EmptySet.AsSpan());
        for (int i = 0; i < BB.Count; i++)
        {
            BitVector p = BB.GetSet(i);
            if (!BitVectorOps.HasFlag(p, CubeFlags.Active))
            {
                continue;
            }

            int dist = CubeDistance.DistanceCapped(cube, BB.GetSpan(i), sRAISE);
            if (dist > 1)
            {
                continue;
            }

            if (dist == 0)
            {
                throw new InvalidOperationException("ON-set and OFF-set are not orthogonal");
            }

            CubeDistance.FindDisjointParts(cube, sxlower, BB.GetSpan(i), sRAISE);
            BB.ActiveCount--;
            BitVectorOps.ClearFlag(p, CubeFlags.Active);
        }

        if (!BitVectorOps.IsEmpty(sxlower))
        {
            BitVectorOps.AndNot(sFREESET, sFREESET, sxlower);
            EliminateLowering(cube, BB, CC, RAISE, FREESET);
        }
    }

    private static void EliminateLowering(CubeData cube, BitVectorFamily BB, BitVectorFamily? CC, BitVector RAISE, BitVector FREESET)
    {
        Span<uint> sRAISE = RAISE.AsSpan(), sFREESET = FREESET.AsSpan();
        Span<uint> sr = cube.Temp![0].AsSpan();
        BitVectorOps.Or(sr, sRAISE, sFREESET);
        for (int i = 0; i < BB.Count; i++)
        {
            BitVector p = BB.GetSet(i);
            if (!BitVectorOps.HasFlag(p, CubeFlags.Active))
            {
                continue;
            }

            if (!CubeDistance.AreDistance0(cube, BB.GetSpan(i), sr))
            {
                BB.ActiveCount--;
                BitVectorOps.ClearFlag(p, CubeFlags.Active);
            }
        }

        if (CC != null)
        {
            for (int i = 0; i < CC.Count; i++)
            {
                BitVector p = CC.GetSet(i);
                if (!BitVectorOps.HasFlag(p, CubeFlags.Active))
                {
                    continue;
                }

                if (!BitVectorOps.IsSubsetOf(CC.GetSpan(i), sr))
                {
                    CC.ActiveCount--;
                    BitVectorOps.ClearFlag(p, CubeFlags.Active);
                }
            }
        }
    }

    private static int SelectMostFrequent(CubeData cube, BitVectorFamily? CC, BitVector FREESET, int[] count)
    {
        Span<uint> sFREESET = FREESET.AsSpan();
        Array.Clear(count);
        if (CC != null)
        {
            for (int j = 0; j < CC.Count; j++)
            {
                if (BitVectorOps.HasFlag(CC.GetSet(j), CubeFlags.Active))
                {
                    ReadOnlySpan<uint> acSpan = CC.GetSpan(j);
                    for (int aci = 0; aci < acSpan.Length; aci++)
                    {
                        uint acVal = acSpan[aci];
                        int acb = aci << BitVectorOps.LogBpi;
                        while (acVal != 0)
                        {
                            count[acb + System.Numerics.BitOperations.TrailingZeroCount(acVal)] += 1;
                            acVal &= acVal - 1;
                        }
                    }
                }
            }
        }

        int best_count = -1, best_part = -1;
        for (int i = 0; i < cube.Size; i++)
        {
            if (BitVectorOps.Contains(sFREESET, i) && count[i] > best_count)
            {
                best_part = i;
                best_count = count[i];
            }
        }

        return best_part;
    }
}
