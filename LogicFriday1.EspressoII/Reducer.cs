namespace Espresso;

public static class Reducer
{
    private const int TRUE = 1, MAYBE = 0;
    public static BitVector ReduceOneCube(CubeData cube, CubeList FD, BitVector p, SplitStack stack)
    {
        ReadOnlySpan<uint> sp = p.AsSpan();
        var cof = Cofactor.ComputeCofactor(cube, FD, sp);
        BitVector cunder = ContainmentCube(cube, cof, stack, 0);
        cof.ReturnCubes();
        BitVectorOps.And(cunder.AsSpan(), cunder.AsSpan(), sp);
        return cunder;
    }
    private static BitVector ContainmentCube(CubeData cube, CubeList T, SplitStack stack, int depth)
    {
        BitVector r;
        SplitSummary summary;
        int ccscResult;
        // --- inlined ContainmentCubeSpecialCases ---
        {
            summary = default;
            Span<uint> stemp = cube.Temp[1].AsSpan();
            ReadOnlySpan<uint> scof = T.CofSpan, sFull = cube.FullSet.AsSpan();
            r = BitVector.Null;
            ccscResult = MAYBE;
            if (T.Count == 0)
            {
                r = cube.RentCofCopy(cube.FullSet);
                ccscResult = TRUE;
            }
            if (ccscResult == MAYBE)
            {
                for (int t1 = 0; t1 < T.Count; t1++)
                    if (CubeDistance.IsFullCoverage(cube, T.GetSpan(t1), scof))
                    {
                        r = cube.RentCofEmpty();
                        ccscResult = TRUE;
                        break;
                    }
            }
            if (ccscResult == MAYBE)
            {
                summary = Cofactor.AnalyzeSplitVariable(cube, T);
                if (summary.VarsUnate == summary.VarsActive || T.Count <= 1)
                {
                    r = cube.RentCofCopy(cube.FullSet);
                    for (int t1 = 0; t1 < T.Count; t1++)
                    {
                        BitVectorOps.Or(stemp, T.GetSpan(t1), scof);
                        SingleCubeContainment(cube, r, cube.Temp[1]);
                    }
                    ccscResult = TRUE;
                }
            }
            if (ccscResult == MAYBE)
            {
                BitVector ceil = cube.Temp[3];
                Span<uint> sceil = ceil.AsSpan();
                BitVectorOps.Copy(sceil, T.CofSpan);
                for (int t1 = 0; t1 < T.Count; t1++)
                    BitVectorOps.Or(sceil, sceil, T.GetSpan(t1));
                if (!BitVectorOps.AreEqual(sceil, sFull))
                {
                    r = SingleCubeContainment(cube, cube.RentCofCopy(cube.FullSet), ceil);
                    Span<uint> sr = r.AsSpan();
                    if (!BitVectorOps.AreEqual(sr, sFull))
                    {
                        var cof = Cofactor.ComputeCofactor(cube, T, sceil);
                        var leftArg = ContainmentCube(cube, cof, stack, depth);
                        var rightArg = cube.RentCofCopy(cube.FullSet);
                        var oldR = r;
                        r = MergeContainmentCubes(cube, leftArg, rightArg, ceil, oldR);
                        cube.ReturnCof(oldR);
                        cof.ReturnCubes();
                    }
                    ccscResult = TRUE;
                }
                else if (summary.VarsActive == 1)
                {
                    r = cube.RentCofEmpty();
                    ccscResult = TRUE;
                }
                else if (summary.BestVarZeros < T.Count / 2)
                {
                    if (CoverManipulation.PartitionCubeList(cube, T, out CubeList A, out CubeList B) != 0)
                    {
                        r = ContainmentCube(cube, A, stack, depth);
                        var rB = ContainmentCube(cube, B, stack, depth);
                        BitVectorOps.And(r.AsSpan(), r.AsSpan(), rB.AsSpan());
                        cube.ReturnCof(rB);
                        A.ReturnCubes(); B.ReturnCubes();
                        ccscResult = TRUE;
                    }
                }
            }
        }
        // --- end inlined ContainmentCubeSpecialCases ---
        if (ccscResult == MAYBE)
        {
            stack.GetPair(depth, out BitVector cl, out BitVector cr);
            Span<uint> scl = cl.AsSpan(), scr = cr.AsSpan();
            int best = summary.Best;
            Cofactor.BuildSplitCubes(cube, T, best, scl, scr);
            var cofL = Cofactor.SingleVariableCofactor(cube, T, scl, best);
            var cofR = Cofactor.SingleVariableCofactor(cube, T, scr, best);
            r = MergeContainmentCubes(cube,
                ContainmentCube(cube, cofL, stack, depth + 1),
                ContainmentCube(cube, cofR, stack, depth + 1),
                cl, cr);
            cofL.ReturnCubes();
            cofR.ReturnCubes();
        }
        return r;
    }
    private static BitVector MergeContainmentCubes(CubeData cube, BitVector left, BitVector right, BitVector cl, BitVector cr)
    {
        Span<uint> sleft = left.AsSpan(), sright = right.AsSpan();
        BitVectorOps.And(sleft, sleft, cl.AsSpan());
        BitVectorOps.And(sright, sright, cr.AsSpan());
        BitVectorOps.Or(sleft, sleft, sright);
        cube.ReturnCof(right);
        return left;
    }
    private static BitVector SingleCubeContainment(CubeData cube, BitVector result, BitVector p)
    {
        Span<uint> stemp = cube.Temp[0].AsSpan(), sresult = result.AsSpan();
        ReadOnlySpan<uint> sp = p.AsSpan();
        // --- inlined SingleActiveVariable ---
        int var;
        {
            int savActive = -1, savDist = 0, savLast = cube.InWord;
            if (savLast != -1)
            {
                uint x = sp[savLast];
                x = ~(x & (x >> 1)) & cube.InMask;
                if (x != 0)
                {
                    savDist = BitVectorOps.CountOnes(x);
                    if (savDist > 1) { var = -1; goto savDone; }
                    savActive = savLast * (BitVectorOps.Bpi / 2) + BitVectorOps.BitIndex(x) / 2;
                }
                for (int w = 0; w < savLast; w++)
                {
                    x = sp[w];
                    x = ~(x & (x >> 1)) & BitVectorOps.Disjoint;
                    if (x != 0)
                    {
                        savDist += BitVectorOps.CountOnes(x);
                        if (savDist > 1) { var = -1; goto savDone; }
                        savActive = w * (BitVectorOps.Bpi / 2) + BitVectorOps.BitIndex(x) / 2;
                    }
                }
            }
            for (int savVar = cube.NumBinaryVars; savVar < cube.NumVars; savVar++)
            {
                ReadOnlySpan<uint> sm = cube.VarMask[savVar].AsSpan();
                int mvLast = cube.LastWordOf(savVar);
                for (int w = cube.FirstWordOf(savVar); w <= mvLast; w++)
                    if ((sm[w] & ~sp[w]) != 0) { if (++savDist > 1) { var = -1; goto savDone; } savActive = savVar; break; }
            }
            var = savActive;
            savDone:;
        }
        // --- end inlined SingleActiveVariable ---
        if (var >= 0)
        {
            BitVectorOps.Xor(stemp, sp, cube.VarMask[var].AsSpan());
            BitVectorOps.And(sresult, sresult, stemp);
        }
        return result;
    }
}