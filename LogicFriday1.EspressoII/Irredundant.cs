using System.Buffers;

namespace Espresso;

public static class Irredundant
{
    private const int TautFalse = 0, TautTrue = 1, TautMaybe = 2;

    public static BitVectorFamily FindIrredundant(CubeData cube, BitVectorFamily F, BitVectorFamily D, SplitStack stack)
    {
        MemoCache.Key key = default;
        bool cacheActive = MemoCache.Enabled;
        if (cacheActive)
        {
            key = MemoCache.BuildFamiliesKey(MemoCache.TagFindIrredundant, cube, F, D, null, 0);
            if (MemoCache.TryGetFamily(key, cube.Size, out var cached))
            {
                return cached;
            }
        }

        MarkIrredundant(cube, F, D, stack);
        var result = BitVectorFamily.CompactInactive(F);
        if (cacheActive)
        {
            MemoCache.PutFamily(key, result);
        }

        return result;
    }

    public static void MarkIrredundant(CubeData cube, BitVectorFamily F, BitVectorFamily D, SplitStack stack)
    {
        // --- inlined SplitCoverByRedundancy ---
        BitVectorFamily E, Rp;
        {
            for (int si = 0; si < F.Count; si++)
            {
                BitVectorOps.SetSortKey(F.GetSet(si), si);
            }

            int cap = Math.Max(F.Count / 2, 4);
            E = BitVectorFamily.Create(cap, F.SfSize);
            Rp = BitVectorFamily.Create(cap, F.SfSize);
            var R = BitVectorFamily.Create(cap, F.SfSize);
            CubeList FD = Cofactor.BuildCubeList(cube, F, D);
            for (int si = 0; si < F.Count; si++)
            {
                BitVector p = F.GetSet(si);
                if (IsCubeCovered(cube, FD, p, stack))
                {
                    R = BitVectorFamily.Add(R, p);
                }
                else
                {
                    E = BitVectorFamily.Add(E, p);
                }
            }

            FD.ReturnCubes();
            CubeList ED = Cofactor.BuildCubeList(cube, E, D);
            for (int si = 0; si < R.Count; si++)
            {
                BitVector p = R.GetSet(si);
                if (!IsCubeCovered(cube, ED, p, stack))
                {
                    Rp = BitVectorFamily.Add(Rp, p);
                }
            }

            ED.ReturnCubes();
        }

        // --- inlined DeriveCoverTable ---
        SparseMatrix coverTable;
        {
            BitVectorFamily.ClearAllFlags(D, CubeFlags.Redund);
            BitVectorFamily.ClearAllFlags(E, CubeFlags.Redund);
            BitVectorFamily.SetAllFlags(Rp, CubeFlags.Redund);
            var list = Cofactor.BuildCubeList(cube, D, E, Rp);
            coverTable = new SparseMatrix();
            for (int j = 0; j < Rp.Count; j++)
            {
                BitVector p = Rp.GetSet(j);
                var cof = Cofactor.ComputeCofactor(cube, list, Rp.GetSpan(j));
                FunctionalTautology(cube, cof, coverTable, BitVectorOps.GetSortKey(p), stack, 0);
                cof.ReturnCubes();
                BitVectorOps.ClearFlag(p, CubeFlags.Redund);
            }

            list.ReturnCubes();
        }

        var cover = MinimumCoverSolver.Solve(coverTable);
        BitVectorFamily.ClearAllFlags(F, CubeFlags.Active | CubeFlags.RelEssen);
        for (int i = 0; i < E.Count; i++)
        {
            BitVector p1 = F.GetSet(BitVectorOps.GetSortKey(E.GetSet(i)));
            BitVectorOps.AddFlag(p1, CubeFlags.Active);
            BitVectorOps.AddFlag(p1, CubeFlags.RelEssen);
        }

        foreach (int col in cover.Cols)
        {
            BitVectorOps.AddFlag(F.GetSet(col), CubeFlags.Active);
        }
    }

    public static bool IsCubeCovered(CubeData cube, CubeList T, BitVector c, SplitStack stack)
    {
        // NOTE: not safely cacheable — Cofactor.ComputeCofactor filters via sp.Overlaps(c) which
        // is a memory-alias check, so the result depends on whether c is a BitVector reference
        // owned by T or just a same-data copy. IsTautology (invoked below) is pure and caches itself.
        var cof = Cofactor.ComputeCofactor(cube, T, c.AsSpan());
        bool result = IsTautology(cube, cof, stack, 0);
        cof.ReturnCubes();
        return result;
    }

    public static bool IsTautology(CubeData cube, CubeList T, SplitStack stack, int depth)
    {
        MemoCache.Key tautKey = default;
        bool tautCacheActive = MemoCache.Enabled && Environment.GetEnvironmentVariable("ESPRESSO_CACHE_NO_TAUT") != "1";
        if (tautCacheActive)
        {
            tautKey = MemoCache.BuildCubeListKey(MemoCache.TagIsTautology, cube, T);
            if (MemoCache.TryGetBool(tautKey, out bool cached))
            {
                return cached;
            }
        }

        SplitSummary summary;
        int tscResult;
        // --- inlined TautologySpecialCases ---
        {
            summary = default;
            Span<uint> sceil = cube.Temp[0].AsSpan();
            ReadOnlySpan<uint> scof = T.CofSpan, sFull = cube.FullSet.AsSpan();
            bool firstPass = true;
            tscResult = TautMaybe;
            while (true)
            {
                BitVectorOps.Copy(sceil, scof);
                for (int ti = 0; ti < T.Count; ti++)
                {
                    ReadOnlySpan<uint> sp = T.GetSpan(ti);
                    if (firstPass && CubeDistance.IsFullCoverage(cube, sp, scof))
                    {
                        tscResult = TautTrue;
                        break;
                    }

                    BitVectorOps.Or(sceil, sceil, sp);
                }

                if (tscResult != TautMaybe)
                {
                    break;
                }

                firstPass = false;
                if (!BitVectorOps.AreEqual(sceil, sFull))
                {
                    tscResult = TautFalse;
                    break;
                }

                summary = Cofactor.AnalyzeSplitVariable(cube, T);
                if (summary.VarsUnate == summary.VarsActive)
                {
                    tscResult = TautFalse;
                    break;
                }

                if (summary.VarsActive == 1)
                {
                    tscResult = TautTrue;
                    break;
                }

                if (summary.VarsUnate != 0)
                {
                    var analysis = Cofactor.AnalyzeAllVariables(cube, T);
                    T = FilterUnate(cube, T, analysis, cube.Temp[0], cube.Temp[1]);
                    continue;
                }

                if (summary.BestVarZeros < T.Count / 2)
                {
                    if (CoverManipulation.PartitionCubeList(cube, T, out CubeList A, out CubeList B) == 0)
                    {
                        break; // TautMaybe
                    }

                    bool rp = IsTautology(cube, A, stack, depth) ? true : IsTautology(cube, B, stack, depth);
                    A.ReturnCubes();
                    B.ReturnCubes();
                    if (tautCacheActive)
                    {
                        MemoCache.PutBool(tautKey, rp);
                    }

                    return rp;
                }

                break; // TautMaybe
            }
        }

        // --- end inlined TautologySpecialCases ---
        if (tscResult != TautMaybe)
        {
            bool r = tscResult == TautTrue;
            if (tautCacheActive)
            {
                MemoCache.PutBool(tautKey, r);
            }

            return r;
        }

        stack.GetPair(depth, out BitVector cl, out BitVector cr);
        Span<uint> scl = cl.AsSpan(), scr = cr.AsSpan();
        int best = summary.Best;
        Cofactor.BuildSplitCubes(cube, T, best, scl, scr);
        var cofL = Cofactor.SingleVariableCofactor(cube, T, scl, best);
        var cofR = Cofactor.SingleVariableCofactor(cube, T, scr, best);
        bool result2 = IsTautology(cube, cofL, stack, depth + 1)
            && IsTautology(cube, cofR, stack, depth + 1);
        cofL.ReturnCubes();
        cofR.ReturnCubes();
        if (tautCacheActive)
        {
            MemoCache.PutBool(tautKey, result2);
        }

        return result2;
    }

    private static void FunctionalTautology(CubeData cube, CubeList T, SparseMatrix table, int rpCurrent, SplitStack stack, int depth)
    {
        SplitSummary summary;
        int ftscResult;
        // --- inlined FunctionalTautologySpecialCases ---
        {
            summary = default;
            ReadOnlySpan<uint> scof = T.CofSpan;
            ftscResult = TautMaybe;
            while (true)
            {
                for (int fi = 0; fi < T.Count; fi++)
                {
                    if (!BitVectorOps.HasFlag(T[fi], CubeFlags.Redund) && CubeDistance.IsFullCoverage(cube, T.GetSpan(fi), scof))
                    {
                        ftscResult = TautTrue;
                        break;
                    }
                }

                if (ftscResult != TautMaybe)
                {
                    break;
                }

                summary = Cofactor.AnalyzeSplitVariable(cube, T);
                if (summary.VarsUnate == summary.VarsActive)
                {
                    int rownum = table.LastRowNum + 1;
                    SparseMatrix.Insert(table, rownum, rpCurrent);
                    for (int fi = 0; fi < T.Count; fi++)
                    {
                        if (BitVectorOps.HasFlag(T[fi], CubeFlags.Redund) && CubeDistance.IsFullCoverage(cube, T.GetSpan(fi), scof))
                        {
                            SparseMatrix.Insert(table, rownum, BitVectorOps.GetSortKey(T[fi]));
                        }
                    }

                    ftscResult = TautTrue;
                    break;
                }

                if (summary.VarsUnate != 0)
                {
                    var analysis = Cofactor.AnalyzeAllVariables(cube, T);
                    T = FilterUnate(cube, T, analysis, cube.Temp[1], cube.Temp[0]);
                    scof = T.CofSpan;
                    continue;
                }

                break; // TautMaybe
            }
        }

        // --- end inlined FunctionalTautologySpecialCases ---
        if (ftscResult == TautMaybe)
        {
            stack.GetPair(depth, out BitVector cl, out BitVector cr);
            Span<uint> scl = cl.AsSpan(), scr = cr.AsSpan();
            int best = summary.Best;
            Cofactor.BuildSplitCubes(cube, T, best, scl, scr);
            var cofL = Cofactor.SingleVariableCofactor(cube, T, scl, best);
            var cofR = Cofactor.SingleVariableCofactor(cube, T, scr, best);
            FunctionalTautology(cube, cofL, table, rpCurrent, stack, depth + 1);
            FunctionalTautology(cube, cofR, table, rpCurrent, stack, depth + 1);
            cofL.ReturnCubes();
            cofR.ReturnCubes();
        }
    }

    private static CubeList FilterUnate(CubeData cube, CubeList T, VariableAnalysis analysis, BitVector ceil, BitVector temp)
    {
        Span<uint> sceil = ceil.AsSpan(), stemp = temp.AsSpan();
        ReadOnlySpan<uint> scof = T.CofSpan;
        BitVectorOps.Copy(sceil, cube.EmptySet.AsSpan());
        for (int v = 0; v < cube.NumVars; v++)
        {
            if (analysis.IsUnate[v])
            {
                BitVectorOps.Or(sceil, sceil, cube.VarMask[v].AsSpan());
            }
        }

        Cofactor.ReturnAnalysis(analysis);
        var filtered = ArrayPool<BitVector>.Shared.Rent(Math.Max(T.Count, 1));
        int fc = 0;
        for (int i = 0; i < T.Count; i++)
        {
            BitVectorOps.Or(stemp, T.GetSpan(i), scof);
            if (BitVectorOps.IsSubsetOf(sceil, stemp))
            {
                filtered[fc++] = T[i];
            }
        }

        return new CubeList(T.Cof, filtered, fc, rented: true);
    }
}
