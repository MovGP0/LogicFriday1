using System.Buffers;

namespace Espresso;

public static class Complement
{
    private const int UseComplLift = 0, UseComplLiftOnset = 1;

    public static BitVectorFamily ComputeComplement(CubeData cube, CubeList T)
    {
        MemoCache.Key key = default;
        bool cacheActive = MemoCache.Enabled;
        if (cacheActive)
        {
            key = MemoCache.BuildCubeListFamilyKey(MemoCache.TagComplement, cube, T);
            if (MemoCache.TryGetFamily(key, cube.Size, out var cached))
            {
                return cached;
            }
        }

        var result = ComputeComplement(cube, T, new SplitStack(cube.Size, cube.Size), 0);
        if (cacheActive)
        {
            MemoCache.PutFamily(key, result);
        }

        return result;
    }

    private static BitVectorFamily ComputeComplement(CubeData cube, CubeList T, SplitStack stack, int depth)
    {
        BitVectorFamily Tbar;
        bool cscHandled = false;
        // --- inlined ComplementSpecialCases ---
        {
            BitVector cof = T.Cof;
            ReadOnlySpan<uint> scof = T.CofSpan, sFull = cube.FullSet.AsSpan();
            if (T.Count == 0)
            {
                Tbar = BitVectorFamily.Add(BitVectorFamily.Create(1, cube.Size), cube.FullSet);
                cscHandled = true;
            }
            else if (T.Count == 1)
            {
                BitVector tmp = BitVectorOps.Clone(cof);
                BitVectorOps.Or(tmp.AsSpan(), scof, T.GetSpan(0));
                Tbar = ComplementSingleCube(cube, tmp);
                cscHandled = true;
            }
            else
            {
                Tbar = default!;
                for (int t1 = 0; t1 < T.Count; t1++)
                {
                    if (CubeDistance.IsFullCoverage(cube, T.GetSpan(t1), scof))
                    {
                        Tbar = BitVectorFamily.Create(0, cube.Size);
                        cscHandled = true;
                        break;
                    }
                }

                if (!cscHandled)
                {
                    BitVector ceil = BitVectorOps.Clone(cof);
                    Span<uint> sceil = ceil.AsSpan();
                    for (int t1 = 0; t1 < T.Count; t1++)
                    {
                        BitVectorOps.Or(sceil, sceil, T.GetSpan(t1));
                    }

                    if (!BitVectorOps.AreEqual(sceil, sFull))
                    {
                        BitVectorFamily ceilCompl = ComplementSingleCube(cube, ceil);
                        BitVectorOps.AndNot(sceil, sFull, sceil);
                        BitVectorOps.Or(cof.AsSpan(), scof, sceil);
                        Tbar = BitVectorFamily.Append(ComputeComplement(cube, T, stack, depth), ceilCompl);
                        cscHandled = true;
                    }
                    else
                    {
                        var analysis = Cofactor.AnalyzeAllVariables(cube, T);
                        if (analysis.VarsActive == 1)
                        {
                            Cofactor.ReturnAnalysis(analysis);
                            Tbar = BitVectorFamily.Create(0, cube.Size);
                            cscHandled = true;
                        }
                        else if (analysis.VarsUnate == analysis.VarsActive)
                        {
                            // --- inlined MapToUnate + Compute + MapFromUnate ---
                            BitVectorFamily ucA;
                            {
                                ucA = BitVectorFamily.Create(T.Count, analysis.VarsUnate);
                                ucA.Count = T.Count;
                                for (int i = 0; i < ucA.Count; i++)
                                {
                                    ucA.GetSpan(i).Clear();
                                }

                                int ncol = 0;
                                for (int i = 0; i < cube.Size; i++)
                                {
                                    if (analysis.PartZeros[i] <= 0)
                                    {
                                        continue;
                                    }

                                    int wordTest = BitVectorOps.WhichWord(i), bitTest = BitVectorOps.WhichBit(i);
                                    int wordSet = BitVectorOps.WhichWord(ncol), bitSet = BitVectorOps.WhichBit(ncol);
                                    for (int j = 0; j < T.Count; j++)
                                    {
                                        if ((T.GetSpan(j)[wordTest] & (1u << bitTest)) == 0)
                                        {
                                            ucA.GetSpan(j)[wordSet] |= (uint)(1 << bitSet);
                                        }
                                    }

                                    ncol++;
                                }
                            }

                            // Compute
                            {
                                for (int si = 0; si < ucA.Count; si++)
                                {
                                    BitVectorOps.SetSortKey(ucA.GetSet(si), BitVectorOps.PopCount(ucA.GetSpan(si)));
                                }

                                ucA = UnateComplement.ComplementRecursive(ucA);
                                if (ucA.Count > 0)
                                {
                                    for (int i = 0; i < ucA.Count; i++)
                                    {
                                        BitVectorOps.SetSortKey(ucA.GetSet(i), BitVectorOps.PopCount(ucA.GetSpan(i)));
                                    }

                                    int[] ucOrder = ArrayPool<int>.Shared.Rent(ucA.Count);
                                    for (int i = 0; i < ucA.Count; i++)
                                    {
                                        ucOrder[i] = i;
                                    }

                                    ucOrder.AsSpan(0, ucA.Count).Sort((a, b) =>
                                    {
                                        int sa = BitVectorOps.GetSortKey(ucA.GetSet(a)), sb = BitVectorOps.GetSortKey(ucA.GetSet(b));
                                        return sa != sb ? sa - sb : BitVectorOps.CompareAscending(ucA.GetSet(a), ucA.GetSet(b));
                                    });
                                    bool[] ucKeep = ArrayPool<bool>.Shared.Rent(ucA.Count);
                                    Array.Fill(ucKeep, true, 0, ucA.Count);
                                    for (int i = 0; i < ucA.Count; i++)
                                    {
                                        if (!ucKeep[ucOrder[i]])
                                        {
                                            continue;
                                        }

                                        int iPop = BitVectorOps.PopCount(ucA.GetSpan(ucOrder[i]));
                                        for (int j = i + 1; j < ucA.Count; j++)
                                        {
                                            if (!ucKeep[ucOrder[j]])
                                            {
                                                continue;
                                            }

                                            if (BitVectorOps.PopCount(ucA.GetSpan(ucOrder[j])) < iPop)
                                            {
                                                continue;
                                            }

                                            if (BitVectorOps.IsSubsetOf(ucA.GetSpan(ucOrder[i]), ucA.GetSpan(ucOrder[j])))
                                            {
                                                ucKeep[ucOrder[j]] = false;
                                            }
                                        }
                                    }

                                    int ucCnt = 0;
                                    for (int i = 0; i < ucA.Count; i++)
                                    {
                                        if (ucKeep[ucOrder[i]])
                                        {
                                            ucCnt++;
                                        }
                                    }

                                    var ucR = BitVectorFamily.Create(ucCnt, ucA.SfSize);
                                    for (int i = 0; i < ucA.Count; i++)
                                    {
                                        if (!ucKeep[ucOrder[i]])
                                        {
                                            continue;
                                        }

                                        Array.Copy(ucA.Data, ucOrder[i] * ucA.Stride, ucR.Data, ucR.Count * ucR.Stride, ucA.Stride);
                                        ucR.Count++;
                                    }

                                    ArrayPool<int>.Shared.Return(ucOrder, clearArray: false);
                                    ArrayPool<bool>.Shared.Return(ucKeep, clearArray: false);
                                    ucA = ucR;
                                }
                            }

                            // MapFromUnate
                            {
                                var ucB = BitVectorFamily.Create(ucA.Count, cube.Size);
                                ucB.Count = ucA.Count;
                                int[] unate = ArrayPool<int>.Shared.Rent(cube.NumVars);
                                int nunate = 0;
                                for (int v = 0; v < cube.NumVars; v++)
                                {
                                    if (analysis.IsUnate[v])
                                    {
                                        unate[nunate++] = v;
                                    }
                                }

                                for (int si = 0; si < ucA.Count; si++)
                                {
                                    ReadOnlySpan<uint> ucsp = ucA.GetSpan(si);
                                    Span<uint> ucspB = ucB.GetSpan(si);
                                    BitVectorOps.Fill(ucspB, cube.Size);
                                    for (int ncol = 0; ncol < nunate; ncol++)
                                    {
                                        if (BitVectorOps.Contains(ucsp, ncol))
                                        {
                                            for (int i = cube.FirstPart![unate[ncol]]; i <= cube.LastPart![unate[ncol]]; i++)
                                            {
                                                if (analysis.PartZeros[i] == 0)
                                                {
                                                    BitVectorOps.Remove(ucspB, i);
                                                }
                                            }
                                        }
                                    }
                                }

                                ArrayPool<int>.Shared.Return(unate, clearArray: false);
                                Tbar = ucB;
                            }

                            // --- end inlined MapToUnate + Compute + MapFromUnate ---
                            Cofactor.ReturnAnalysis(analysis);
                            cscHandled = true;
                        }
                        else
                        {
                            Cofactor.ReturnAnalysis(analysis);
                        }
                    }
                }
            }
        }

        // --- end inlined ComplementSpecialCases ---
        if (cscHandled)
        {
            return Tbar;
        }

        stack.GetPair(depth, out BitVector cl, out BitVector cr);
        Span<uint> scl = cl.AsSpan(), scr = cr.AsSpan();
        var summary = Cofactor.AnalyzeSplitVariable(cube, T);
        int best = summary.Best;
        Cofactor.BuildSplitCubes(cube, T, best, scl, scr);
        var cofL = Cofactor.SingleVariableCofactor(cube, T, scl, best);
        var cofR = Cofactor.SingleVariableCofactor(cube, T, scr, best);
        BitVectorFamily Tl = ComputeComplement(cube, cofL, stack, depth + 1);
        BitVectorFamily Tr = ComputeComplement(cube, cofR, stack, depth + 1);
        cofL.ReturnCubes();
        cofR.ReturnCubes();
        int lifting = Tr.Count * Tl.Count > (Tr.Count + Tl.Count) * T.Count ? UseComplLiftOnset : UseComplLift;

        // --- inlined MergeComplements ---
        {
            ReadOnlySpan<uint> smcl = cl.AsSpan(), smcr = cr.AsSpan();
            for (int i = 0; i < Tl.Count; i++)
            {
                BitVectorOps.And(Tl.GetSpan(i), Tl.GetSpan(i), smcl);
                BitVectorOps.AddFlag(Tl.GetSet(i), CubeFlags.Active);
            }

            for (int i = 0; i < Tr.Count; i++)
            {
                BitVectorOps.And(Tr.GetSpan(i), Tr.GetSpan(i), smcr);
                BitVectorOps.AddFlag(Tr.GetSet(i), CubeFlags.Active);
            }

            BitVectorOps.Copy(cube.Temp[0].AsSpan(), cube.VarMask[best].AsSpan());
            BitVector[] L1 = SfListSorted(cube, Tl), R1 = SfListSorted(cube, Tr);
            {
                int li = 0, ri = 0;
                while (li < L1.Length && ri < R1.Length)
                {
                    switch (CubeDistance.Distance1Order(cube, L1[li], R1[ri]))
                    {
                        case 1:
                            ri++;
                            break;
                        case -1:
                            li++;
                            break;
                        default:
                            BitVectorOps.ClearFlag(R1[ri], CubeFlags.Active);
                            Span<uint> sL = L1[li].AsSpan();
                            BitVectorOps.Or(sL, sL, R1[ri].AsSpan());
                            ri++;
                            break;
                    }
                }
            }

            switch (lifting)
            {
                case UseComplLiftOnset:
                    {
                        // --- inlined MergeCubeList ---
                        BitVectorFamily Tcover;
                        {
                            Tcover = BitVectorFamily.Create(T.Count, cube.Size);
                            ReadOnlySpan<uint> mcCofSpan = T.CofSpan;
                            for (int mci = 0; mci < T.Count; mci++)
                            {
                                BitVectorOps.Or(Tcover.GetSpan(mci), T.GetSpan(mci), mcCofSpan);
                            }

                            Tcover.Count = T.Count;
                        }

                        // --- end inlined MergeCubeList ---
                        LiftComplementOnset(cube, L1, Tcover, cr, best);
                        LiftComplementOnset(cube, R1, Tcover, cl, best);
                        break;
                    }
                case UseComplLift:
                    LiftComplement(cube, L1, R1, cr, best);
                    LiftComplement(cube, R1, L1, cl, best);
                    break;
            }

            Tbar = BitVectorFamily.Create(Tl.Count + Tr.Count, cube.Size);
            for (int i = 0; i < Tl.Count; i++)
            {
                BitVectorFamily.Add(Tbar, Tl.GetSet(i));
            }

            for (int i = 0; i < Tr.Count; i++)
            {
                if (BitVectorOps.HasFlag(Tr.GetSet(i), CubeFlags.Active))
                {
                    BitVectorFamily.Add(Tbar, Tr.GetSet(i));
                }
            }
        }

        // --- end inlined MergeComplements ---
        return Tbar;
    }

    private static BitVectorFamily ComplementSingleCube(CubeData cube, BitVector p)
    {
        Span<uint> sdiff = cube.Temp[7].AsSpan();
        ReadOnlySpan<uint> sfull = cube.FullSet.AsSpan(), sp = p.AsSpan();
        BitVectorOps.AndNot(sdiff, sfull, sp);
        var R = BitVectorFamily.Create(cube.NumVars, cube.Size);
        for (int var = 0; var < cube.NumVars; var++)
        {
            ReadOnlySpan<uint> smask = cube.VarMask[var].AsSpan();
            if (!BitVectorOps.AreDisjoint(sdiff, smask))
            {
                BitVectorOps.MergeWithMask(R.GetSet(R.Count++).AsSpan(), sdiff, sfull, smask);
            }
        }

        return R;
    }

    private static void LiftComplement(CubeData cube, BitVector[] A1, BitVector[] B1, BitVector bcube, int var)
    {
        BitVector lift = cube.Temp[4], liftor = cube.Temp[5], mask = cube.VarMask[var];
        Span<uint> slift = lift.AsSpan(), sliftor = liftor.AsSpan();
        ReadOnlySpan<uint> sbcube = bcube.AsSpan(), smask = mask.AsSpan();
        BitVectorOps.And(sliftor, sbcube, smask);
        foreach (var a in A1)
        {
            if (!BitVectorOps.HasFlag(a, CubeFlags.Active))
            {
                continue;
            }

            Span<uint> sa = a.AsSpan();
            BitVectorOps.MergeWithMask(slift, sbcube, sa, smask);
            int liftPop = BitVectorOps.PopCount(slift);
            for (int bi = 0; bi < B1.Length; bi++)
            {
                if (BitVectorOps.PopCount(B1[bi].AsSpan()) < liftPop)
                {
                    continue;
                }

                if (!BitVectorOps.IsSubsetOf(slift, B1[bi].AsSpan()))
                {
                    continue;
                }

                BitVectorOps.Or(sa, sa, sliftor);
                break;
            }
        }
    }

    private static void LiftComplementOnset(CubeData cube, BitVector[] A1, BitVectorFamily T, BitVector bcube, int var)
    {
        BitVector lift = cube.Temp[4], mask = cube.VarMask[var];
        Span<uint> slift = lift.AsSpan();
        ReadOnlySpan<uint> sbcube = bcube.AsSpan(), smask = mask.AsSpan();
        foreach (var a in A1)
        {
            if (!BitVectorOps.HasFlag(a, CubeFlags.Active))
            {
                continue;
            }

            Span<uint> sa = a.AsSpan();
            BitVectorOps.And(slift, sbcube, smask);
            BitVectorOps.Or(slift, sa, slift);
            bool canLift = true;
            for (int ti = 0; ti < T.Count; ti++)
            {
                if (CubeDistance.AreDistance0(cube, T.GetSpan(ti), slift))
                {
                    canLift = false;
                    break;
                }
            }

            if (canLift)
            {
                BitVectorOps.Copy(sa, slift);
                BitVectorOps.AddFlag(a, CubeFlags.Active);
            }
        }
    }

    private static BitVector[] SfListSorted(CubeData cube, BitVectorFamily F)
    {
        var arr = new BitVector[F.Count];
        for (int i = 0; i < F.Count; i++)
        {
            arr[i] = F.GetSet(i);
        }

        arr.AsSpan().Sort((a, b) => CubeDistance.Distance1Order(cube, a, b));
        return arr;
    }
}
