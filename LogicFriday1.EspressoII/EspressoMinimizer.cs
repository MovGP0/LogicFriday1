using System.Buffers;

namespace Espresso;

using static BitVectorOps;
using static BitVectorFamily;
using static CubeDistance;

public static class EspressoMinimizer
{
    public static BitVectorFamily Minimize(CubeData cube, BitVectorFamily F, BitVectorFamily D1, BitVectorFamily R)
    {
        MemoCache.Key minKey = default;
        bool minCacheActive = MemoCache.Enabled;
        if (minCacheActive)
        {
            minKey = MemoCache.BuildMinimizeKey(cube, F, D1, R);
            if (MemoCache.TryGetFamily(minKey, cube.Size, out var cached))
            {
                return cached;
            }
        }

        BitVectorFamily result = MinimizeUncached(cube, F, D1, R);
        if (minCacheActive)
        {
            MemoCache.PutFamily(minKey, result);
        }

        return result;
    }

    private static BitVectorFamily MinimizeUncached(CubeData cube, BitVectorFamily F, BitVectorFamily D1, BitVectorFamily R)
    {
        var stack = new SplitStack(cube.Size, cube.Size);
        bool unwrapOnset = cube.PartSize[cube.NumVars - 1] > 1;

        while (true)
        {
            BitVectorFamily Fsave = Clone(F);
            BitVectorFamily D = Clone(D1);

            if (unwrapOnset)
            {
                CoverManipulation.CalculateCost(cube, F, out CoverCost initialCost);
                bool worthUnwrapping =
                    initialCost.Out != initialCost.Cubes * cube.PartSize[cube.NumVars - 1]
                        && initialCost.Out < 5000;
                if (worthUnwrapping)
                {
                    F = RemoveContained(cube, F);
                }
            }

            ClearAllFlags(F, CubeFlags.Prime);
            F = Expander.ExpandCover(cube, F, R, 0);
            F = Irredundant.FindIrredundant(cube, F, D, stack);

            BitVectorFamily E = FindEssentials(cube, ref F, D, stack);
            D = Join(D, E);

            CoverManipulation.CalculateCost(cube, F, out CoverCost cost);
            bool useSortReduce = true;
            CoverCost bestCost;
            do
            {
                do
                {
                    bestCost = cost;
                    F = ReduceCover(cube, F, D, stack, ref useSortReduce);
                    F = Expander.ExpandCover(cube, F, R, 0);
                    F = Irredundant.FindIrredundant(cube, F, D, stack);
                    CoverManipulation.CalculateCost(cube, F, out cost);
                } while (cost.Cubes < bestCost.Cubes);
                bestCost = cost;
                F = LastGasp(cube, F, D, R, stack);
                CoverManipulation.CalculateCost(cube, F, out cost);
            } while (cost.Cubes < bestCost.Cubes ||
                (cost.Cubes == bestCost.Cubes && cost.Total < bestCost.Total));

            F = Append(F, E);
            F = MakeSparse(cube, F, D1, R, stack);

            if (Fsave.Count >= F.Count)
            {
                return F;
            }

            // Retry without unwrapping when unwrap hurt us.
            F = Fsave;
            unwrapOnset = false;
        }
    }

    // Remove cubes contained in others after expanding the output variable.
    private static BitVectorFamily RemoveContained(CubeData cube, BitVectorFamily F)
    {
        BitVectorFamily expanded = CoverManipulation.ExpandMultiValued(cube, F, cube.NumVars - 1);
        BitVector[] sorted = ToSortedArray(expanded, CompareDescending);
        int len = RmEqual(sorted, expanded.Count, CompareDescending);

        int dest = 0, checkLimit = 0, lastSize = -1;
        for (int i = 0; i < len; i++)
        {
            BitVector a = sorted[i];
            int aKey = GetSortKey(a);
            if (aKey != lastSize)
            {
                lastSize = aKey;
                checkLimit = dest;
            }

            bool contained = false;
            for (int j = 0; j < checkLimit; j++)
            {
                if (GetSortKey(sorted[j]) < aKey)
                {
                    continue;
                }

                if (IsSubsetOf(a.AsSpan(), sorted[j].AsSpan()))
                {
                    contained = true;
                    break;
                }
            }

            if (!contained)
            {
                sorted[dest++] = a;
            }
        }

        BitVectorFamily result = FromSortedArray(sorted, dest, expanded.SfSize);
        ReturnSortedArray(sorted);
        return result;
    }

    // Find essential primes: each prime whose consensus-with-neighbors doesn't cover it is essential.
    // Deactivates essentials in F (compacted on exit) and returns them in E.
    private static BitVectorFamily FindEssentials(CubeData cube, ref BitVectorFamily F, BitVectorFamily D, SplitStack stack)
    {
        ActivateAll(F);
        BitVectorFamily E = Create(Math.Max(10, F.Count / 4), cube.Size);
        BitVectorFamily FD = Join(F, D);
        BitVectorFamily consensusR = Create(FD.Count * 2, cube.Size);

        // Scratch BitVectors reused across the outer loop.
        BitVector consensusTmp = Create(cube.Size);
        BitVector dist0Tmp = Create(cube.Size);
        Span<uint> sConsensus = consensusTmp.AsSpan();
        Span<uint> sDist0 = dist0Tmp.AsSpan();

        for (int fi = 0; fi < F.Count; fi++)
        {
            BitVector fp = F.GetSet(fi);
            if (HasFlag(fp, CubeFlags.NonEssen) || !HasFlag(fp, CubeFlags.RelEssen))
            {
                continue;
            }

            ReadOnlySpan<uint> ec = fp.AsSpan();
            consensusR.Count = 0;
            consensusR.ActiveCount = 0;

            for (int ti = 0; ti < FD.Count; ti++)
            {
                ReadOnlySpan<uint> sp = FD.GetSpan(ti);
                if (sp.Overlaps(ec))
                {
                    continue;
                }

                int d = DistanceCapped(cube, sp, ec);
                if (d == 0)
                {
                    if (IsSubsetOf(sp, ec))
                    {
                        continue;
                    }

                    Span<uint> spDiff = cube.Temp[0].AsSpan();
                    Span<uint> spAnd = cube.Temp[1].AsSpan();
                    AndNot(spDiff, sp, ec);
                    And(spAnd, sp, ec);
                    bool gotOne = false;
                    for (int v = cube.NumBinaryVars; v < cube.NumVars; v++)
                    {
                        ReadOnlySpan<uint> varMask = cube.VarMask[v].AsSpan();
                        if (!AreDisjoint(spDiff, varMask))
                        {
                            MergeWithMask(sDist0, ec, spAnd, varMask);
                            consensusR = Add(consensusR, dist0Tmp);
                            gotOne = true;
                        }
                    }

                    if (!gotOne && cube.NumBinaryVars > 0)
                    {
                        And(sDist0, sp, ec);
                        consensusR = Add(consensusR, dist0Tmp);
                    }
                }
                else if (d == 1)
                {
                    Consensus(cube, sConsensus, sp, ec);
                    consensusR = Add(consensusR, consensusTmp);
                }
            }

            var cubeList = Cofactor.BuildCubeList(cube, consensusR, D);
            bool covered = Irredundant.IsCubeCovered(cube, cubeList, fp, stack);
            cubeList.ReturnCubes();
            if (!covered)
            {
                E = Add(E, fp);
                ClearFlag(fp, CubeFlags.Active);
                F.ActiveCount--;
            }
        }

        F = CompactInactive(F);
        return E;
    }

    // One reduce pass: sort F, then replace each cube with its minimum reduction.
    // Alternates between "by-coverage" and "by-distance-to-largest" sort each call.
    private static BitVectorFamily ReduceCover(CubeData cube, BitVectorFamily F, BitVectorFamily D, SplitStack stack, ref bool useSortReduce)
    {
        if (useSortReduce)
        {
            F = SortForReduction(cube, F);
        }
        else
        {
            F = CoverManipulation.SortByCoverage(cube, F, CompareDescending);
        }

        useSortReduce = !useSortReduce;

        CubeList FD = Cofactor.BuildCubeList(cube, F, D);
        for (int ri = 0; ri < F.Count; ri++)
        {
            BitVector rp = F.GetSet(ri);
            BitVector reduced = Reducer.ReduceOneCube(cube, FD, rp, stack);
            Span<uint> sReduced = reduced.AsSpan();

            if (AreEqual(sReduced, F.GetSpan(ri)))
            {
                AddFlag(rp, CubeFlags.Active);
                AddFlag(rp, CubeFlags.Prime);
            }
            else
            {
                Copy(F.GetSpan(ri), sReduced);
                ClearFlag(rp, CubeFlags.Prime);
                if (IsEmpty(sReduced))
                {
                    ClearFlag(rp, CubeFlags.Active);
                }
                else
                {
                    AddFlag(rp, CubeFlags.Active);
                }
            }

            cube.ReturnCof(reduced);
        }

        FD.ReturnCubes();
        return CompactInactive(F);
    }

    // Sort F so that cubes close to the largest come first.
    private static BitVectorFamily SortForReduction(CubeData cube, BitVectorFamily F)
    {
        if (F.Count == 0)
        {
            return F;
        }

        int bestSize = -1;
        BitVector largest = BitVector.Null;
        for (int si = 0; si < F.Count; si++)
        {
            int size = PopCount(F.GetSpan(si));
            if (size > bestSize)
            {
                largest = F.GetSet(si);
                bestSize = size;
            }
        }

        ReadOnlySpan<uint> largestSpan = largest.AsSpan();
        for (int si = 0; si < F.Count; si++)
        {
            ReadOnlySpan<uint> sp = F.GetSpan(si);
            int key = ((cube.NumVars - CubeDistance.Distance(cube, largestSpan, sp)) << 7)
                + Math.Min(PopCount(sp), 127);
            SetSortKey(F.GetSet(si), key);
        }

        int[] order = ArrayPool<int>.Shared.Rent(F.Count);
        for (int i = 0; i < F.Count; i++)
        {
            order[i] = i;
        }

        order.AsSpan(0, F.Count).Sort((a, b) => CompareDescending(F.GetSet(a), F.GetSet(b)));
        var result = BitVectorFamily.FromSortedOrder(F, order, F.Count);
        ArrayPool<int>.Shared.Return(order, clearArray: false);
        return result;
    }

    // Last-gasp reduction: try super-cubes by pairwise combination; add resulting primes back into F.
    private static BitVectorFamily LastGasp(CubeData cube, BitVectorFamily F, BitVectorFamily D, BitVectorFamily R, SplitStack stack)
    {
        BitVectorFamily lgG = Create(F.Count, F.SfSize);
        {
            CubeList FD = Cofactor.BuildCubeList(cube, F, D);
            for (int lgi = 0; lgi < F.Count; lgi++)
            {
                BitVector lgp = F.GetSet(lgi);
                BitVector reduced = Reducer.ReduceOneCube(cube, FD, lgp, stack);
                if (IsEmpty(reduced.AsSpan()))
                {
                    throw new InvalidOperationException("empty reduction in reduce_gasp");
                }

                if (AreEqual(reduced.AsSpan(), F.GetSpan(lgi)))
                {
                    lgG = Add(lgG, lgp);
                }
                else
                {
                    ClearFlag(reduced, CubeFlags.Prime);
                    lgG = Add(lgG, reduced);
                }

                cube.ReturnCof(reduced);
            }

            FD.ReturnCubes();
        }

        BitVectorFamily lgG1 = Create(10, F.SfSize);
        // Scratch pair (RAISE, temp) sharing a single backing buffer.
        BitVector lgRAISE, lgTemp;
        {
            int words = WordCount(cube.Size);
            int stride = words + 1;
            var pairData = new uint[2 * stride];
            lgRAISE = new BitVector(pairData, 1, words);
            lgTemp = new BitVector(pairData, stride + 1, words);
        }

        Span<uint> sRAISE = lgRAISE.AsSpan(), sTemp = lgTemp.AsSpan();

        for (int c1 = 0; c1 < lgG.Count; c1++)
        {
            ActivateAll(R);
            ActivateAll(lgG);
            for (int c2 = 0; c2 < lgG.Count; c2++)
            {
                BitVector c2p = lgG.GetSet(c2);
                if (c1 == c2 || HasFlag(c2p, CubeFlags.Prime))
                {
                    lgG.ActiveCount--;
                    ClearFlag(c2p, CubeFlags.Active);
                }
            }

            Copy(sRAISE, lgG.GetSpan(c1));

            BitVector lgFREESET = cube.Temp[2];
            Span<uint> sFREESET = lgFREESET.AsSpan();
            AndNot(sFREESET, cube.FullSet.AsSpan(), sRAISE);
            Expander.DetermineEssentialParts(cube, R, lgG, lgRAISE, lgFREESET);

            // Xraise: union of active offsets AND freeset, then add to RAISE.
            Span<uint> xraise = cube.Temp[0].AsSpan();
            Copy(xraise, cube.EmptySet.AsSpan());
            for (int ri = 0; ri < R.Count; ri++)
            {
                if (HasFlag(R.GetSet(ri), CubeFlags.Active))
                {
                    Or(xraise, xraise, R.GetSpan(ri));
                }
            }

            AndNot(xraise, sFREESET, xraise);
            Or(sRAISE, sRAISE, xraise);
            AndNot(sFREESET, sFREESET, xraise);

            int slotOffset = c1 * F.Stride;
            uint[]? savedSlot = null;
            CubeList fdSwapped = default;
            bool fSwapped = false;

            for (int c2 = 0; c2 < lgG.Count; c2++)
            {
                BitVector c2p = lgG.GetSet(c2);
                if (!HasFlag(c2p, CubeFlags.Active))
                {
                    continue;
                }

                if (!IsSubsetOf(lgG.GetSpan(c2), sRAISE) &&
                    !GaspOptimizer.IsFeasiblyCovered(cube, R, c2p, lgRAISE))
                {
                    continue;
                }

                if (!fSwapped)
                {
                    savedSlot = ArrayPool<uint>.Shared.Rent(F.Stride);
                    Array.Copy(F.Data, slotOffset, savedSlot, 0, F.Stride);
                    Array.Copy(lgG.Data, c1 * lgG.Stride, F.Data, slotOffset, F.Stride);
                    fdSwapped = Cofactor.BuildCubeList(cube, F, D);
                    fSwapped = true;
                }

                BitVector essential = Reducer.ReduceOneCube(cube, fdSwapped, F.GetSet(c2), stack);
                if (GaspOptimizer.IsFeasiblyCovered(cube, R, essential, lgRAISE))
                {
                    Or(sTemp, sRAISE, essential.AsSpan());
                    ClearFlag(lgTemp, CubeFlags.Prime);
                    lgG1 = Add(lgG1, lgTemp);
                }

                cube.ReturnCof(essential);
            }

            if (fSwapped)
            {
                Array.Copy(savedSlot!, 0, F.Data, slotOffset, F.Stride);
                ArrayPool<uint>.Shared.Return(savedSlot!, clearArray: false);
                fdSwapped.ReturnCubes();
            }
        }

        // RemoveDuplicates
        {
            BitVector[] sorted = ToSortedArray(lgG1, CompareDescending);
            lgG1 = FromSortedArray(sorted, RmEqual(sorted, lgG1.Count, CompareDescending), lgG1.SfSize);
            BitVectorFamily.ReturnSortedArray(sorted);
        }

        lgG1 = Expander.ExpandCover(cube, lgG1, R, 0);
        if (lgG1.Count != 0)
        {
            F = Irredundant.FindIrredundant(cube, Append(F, lgG1), D, stack);
        }

        return F;
    }

    // Sparsify the output: drop inputs that aren't needed to cover each output.
    private static BitVectorFamily MakeSparse(CubeData cube, BitVectorFamily F, BitVectorFamily D1, BitVectorFamily R, SplitStack stack)
    {
        MemoCache.Key key = default;
        bool cacheActive = MemoCache.Enabled;
        if (cacheActive)
        {
            key = MemoCache.BuildFamiliesKey(MemoCache.TagMakeSparse, cube, F, D1, R, 0);
            if (MemoCache.TryGetFamily(key, cube.Size, out var cached))
            {
                return cached;
            }
        }

        var result = MakeSparseImpl(cube, F, D1, R, stack);
        if (cacheActive)
        {
            MemoCache.PutFamily(key, result);
        }

        return result;
    }

    private static BitVectorFamily MakeSparseImpl(CubeData cube, BitVectorFamily F, BitVectorFamily D1, BitVectorFamily R, SplitStack stack)
    {
        CoverManipulation.CalculateCost(cube, F, out CoverCost bestCost);
        int[] fCubeIdxBuf = ArrayPool<int>.Shared.Rent(Math.Max(256, F.Count));
        while (true)
        {
            Span<int> fCubeIdx = fCubeIdxBuf;
            BitVectorFamily msF1 = Create(F.Count, cube.Size);
            BitVectorFamily msD1 = Create(D1.Count, cube.Size);

            for (int var = 0; var < cube.NumVars; var++)
            {
                if (!cube.IsSparse(var))
                {
                    continue;
                }

                ReadOnlySpan<uint> sVarMask = cube.VarMask[var].AsSpan();

                for (int ii = cube.FirstPart[var]; ii <= cube.LastPart[var]; ii++)
                {
                    msF1.Count = 0;
                    for (int fi = 0; fi < F.Count; fi++)
                    {
                        Span<uint> sp = F.GetSpan(fi);
                        if (!Contains(sp, ii))
                        {
                            continue;
                        }

                        fCubeIdx[msF1.Count] = fi;
                        Span<uint> sp1 = msF1.GetSpan(msF1.Count++);
                        AndNot(sp1, sp, sVarMask);
                        Insert(sp1, ii);
                    }

                    msD1.Count = 0;
                    for (int di = 0; di < D1.Count; di++)
                    {
                        Span<uint> sp = D1.GetSpan(di);
                        if (!Contains(sp, ii))
                        {
                            continue;
                        }

                        Span<uint> sp1 = msD1.GetSpan(msD1.Count++);
                        AndNot(sp1, sp, sVarMask);
                        Insert(sp1, ii);
                    }

                    Irredundant.MarkIrredundant(cube, msF1, msD1, stack);
                    for (int fi = 0; fi < msF1.Count; fi++)
                    {
                        BitVector p1 = msF1.GetSet(fi);
                        if (HasFlag(p1, CubeFlags.Active))
                        {
                            continue;
                        }

                        Span<uint> sp = F.GetSpan(fCubeIdx[fi]);
                        if (var == cube.NumVars - 1 || !IsSubsetOf(sVarMask, sp))
                        {
                            Remove(sp, ii);
                        }

                        ClearFlag(F.GetSet(fCubeIdx[fi]), CubeFlags.Prime);
                    }
                }
            }

            ActivateAll(F);
            for (int var = 0; var < cube.NumVars; var++)
            {
                if (!cube.IsSparse(var))
                {
                    continue;
                }

                ReadOnlySpan<uint> sVarMask = cube.VarMask[var].AsSpan();
                for (int fi = 0; fi < F.Count; fi++)
                {
                    BitVector msp = F.GetSet(fi);
                    if (HasFlag(msp, CubeFlags.Active) && AreDisjoint(F.GetSpan(fi), sVarMask))
                    {
                        ClearFlag(msp, CubeFlags.Active);
                        F.ActiveCount--;
                    }
                }
            }

            if (F.Count != F.ActiveCount)
            {
                F = CompactInactive(F);
            }

            CoverManipulation.CalculateCost(cube, F, out CoverCost cost);
            if (cost.Total == bestCost.Total)
            {
                break;
            }

            bestCost = cost;
            F = Expander.ExpandCover(cube, F, R, 1);
            CoverManipulation.CalculateCost(cube, F, out cost);
            if (cost.Total == bestCost.Total)
            {
                break;
            }

            bestCost = cost;
        }

        ArrayPool<int>.Shared.Return(fCubeIdxBuf, clearArray: false);
        return F;
    }
}
