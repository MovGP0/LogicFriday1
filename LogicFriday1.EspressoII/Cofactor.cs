using System.Buffers;

namespace Espresso;

public static class Cofactor
{
    public static CubeList ComputeCofactor(CubeData cube, CubeList T, ReadOnlySpan<uint> c)
    {
        Span<uint> temp = cube.Temp[0].AsSpan();
        BitVectorOps.AndNot(temp, cube.FullSet.AsSpan(), c);
        BitVector newCube = cube.RentCof();
        BitVectorOps.Or(newCube.AsSpan(), T.CofSpan, temp);
        var cubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(T.Count, 1));
        int count = 0;
        for (int i = 0; i < T.Count; i++)
        {
            ReadOnlySpan<uint> sp = T.GetSpan(i);
            if (!sp.Overlaps(c) && CubeDistance.AreDistance0(cube, sp, c))
            {
                cubes[count++] = T[i];
            }
        }

        return new CubeList(newCube, cubes, count, rented: true, ownsCof: true, owner: cube);
    }

    public static CubeList SingleVariableCofactor(CubeData cube, CubeList T, ReadOnlySpan<uint> c, int var)
    {
        Span<uint> mask = cube.Temp[1].AsSpan();
        BitVectorOps.AndNot(mask, cube.FullSet.AsSpan(), c);
        BitVector newCube = cube.RentCof();
        BitVectorOps.Or(newCube.AsSpan(), T.CofSpan, mask);
        int first = cube.FirstWordOf(var), last = cube.LastWordOf(var);
        BitVectorOps.And(mask, cube.VarMask[var].AsSpan(), c);
        ReadOnlySpan<uint> sm = mask;
        var cubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(T.Count, 1));
        int count = 0;
        for (int i = 0; i < T.Count; i++)
        {
            ReadOnlySpan<uint> sp = T.GetSpan(i);
            if (!sp.Overlaps(c))
            {
                for (int w = first; w <= last; w++)
                {
                    if ((sp[w] & sm[w]) != 0)
                    {
                        cubes[count++] = T[i];
                        break;
                    }
                }
            }
        }

        return new CubeList(newCube, cubes, count, rented: true, ownsCof: true, owner: cube);
    }

    public static SplitSummary AnalyzeSplitVariable(CubeData cube, CubeList T)
    {
        Span<int> buffer = cube.Size <= 256 ? stackalloc int[cube.Size] : new int[cube.Size];
        buffer.Clear();
        FillPartZeros(T, cube, buffer);
        return Summarize(cube, buffer, null, null);
    }

    public static VariableAnalysis AnalyzeAllVariables(CubeData cube, CubeList T)
    {
        int[] partZeros = ArrayPool<int>.Shared.Rent(cube.Size);
        partZeros.AsSpan(0, cube.Size).Clear();
        FillPartZeros(T, cube, partZeros.AsSpan(0, cube.Size));
        int[] varZeros = ArrayPool<int>.Shared.Rent(cube.NumVars);
        varZeros.AsSpan(0, cube.NumVars).Clear();
        bool[] isUnate = ArrayPool<bool>.Shared.Rent(cube.NumVars);
        isUnate.AsSpan(0, cube.NumVars).Clear();
        var summary = Summarize(cube, partZeros.AsSpan(0, cube.Size), varZeros, isUnate);
        return new VariableAnalysis(partZeros, varZeros, isUnate, summary.VarsActive, summary.VarsUnate, summary.Best);
    }

    public static void ReturnAnalysis(VariableAnalysis a)
    {
        ArrayPool<int>.Shared.Return(a.PartZeros, clearArray: false);
        ArrayPool<int>.Shared.Return(a.VarZeros, clearArray: false);
        ArrayPool<bool>.Shared.Return(a.IsUnate, clearArray: false);
    }

    private static void FillPartZeros(CubeList T, CubeData cube, Span<int> partZeros)
    {
        ReadOnlySpan<uint> sc = T.CofSpan, sf = cube.FullSet.AsSpan();
        for (int t1 = 0; t1 < T.Count; t1++)
        {
            ReadOnlySpan<uint> sp = T.GetSpan(t1);
            for (int i = sp.Length - 1; i >= 0; i--)
            {
                uint val = sf[i] & ~(sp[i] | sc[i]);
                if (val == 0)
                {
                    continue;
                }

                int cb = i << BitVectorOps.LogBpi;
                while (val != 0)
                {
                    partZeros[cb + System.Numerics.BitOperations.TrailingZeroCount(val)]++;
                    val &= val - 1;
                }
            }
        }
    }

    private static SplitSummary Summarize(CubeData cube, ReadOnlySpan<int> partZeros, int[]? varZeros, bool[]? isUnate)
    {
        int best = -1, mostActive = 0, mostZero = 0, mostBalanced = 32000, varsUnate = 0, varsActive = 0;
        for (int var = 0; var < cube.NumVars; var++)
        {
            int active, maxActive, zeroCount;
            if (var < cube.NumBinaryVars)
            {
                int ii = partZeros[var * 2], lastbit = partZeros[var * 2 + 1];
                active = (ii > 0 ? 1 : 0) + (lastbit > 0 ? 1 : 0);
                zeroCount = ii + lastbit;
                maxActive = Math.Max(ii, lastbit);
            }
            else
            {
                active = maxActive = zeroCount = 0;
                int lastbit = cube.LastPart[var];
                for (int i = cube.FirstPart[var]; i <= lastbit; i++)
                {
                    zeroCount += partZeros[i];
                    active += partZeros[i] > 0 ? 1 : 0;
                    if (active > maxActive)
                    {
                        maxActive = active;
                    }
                }
            }

            if (varZeros != null)
            {
                varZeros[var] = zeroCount;
            }

            if (active > mostActive)
            {
                best = var;
                mostActive = active;
                mostZero = zeroCount;
                mostBalanced = maxActive;
            }
            else if (active == mostActive)
            {
                if (zeroCount > mostZero)
                {
                    best = var;
                    mostZero = zeroCount;
                    mostBalanced = maxActive;
                }
                else if (zeroCount == mostZero && maxActive < mostBalanced)
                {
                    best = var;
                    mostBalanced = maxActive;
                }
            }

            if (isUnate != null)
            {
                isUnate[var] = (active == 1);
            }

            varsActive += active > 0 ? 1 : 0;
            varsUnate += active == 1 ? 1 : 0;
        }

        return new SplitSummary(varsActive, varsUnate, best, mostZero);
    }

    public static void BuildSplitCubes(CubeData cube, CubeList T, int best, Span<uint> cleft, Span<uint> cright)
    {
        int lastbit = cube.LastPart[best];
        ReadOnlySpan<uint> cof = T.CofSpan, full = cube.FullSet.AsSpan(), vmask = cube.VarMask[best].AsSpan();
        BitVectorOps.AndNot(cleft, full, vmask);
        BitVectorOps.AndNot(cright, full, vmask);
        int halfbit = 0;
        for (int i = cube.FirstPart[best]; i <= lastbit; i++)
        {
            if (!BitVectorOps.Contains(cof, i))
            {
                halfbit++;
            }
        }

        halfbit /= 2;
        int j = cube.FirstPart[best];
        for (; halfbit > 0; j++)
        {
            if (!BitVectorOps.Contains(cof, j))
            {
                halfbit--;
                BitVectorOps.Insert(cleft, j);
            }
        }

        for (; j <= lastbit; j++)
        {
            if (!BitVectorOps.Contains(cof, j))
            {
                BitVectorOps.Insert(cright, j);
            }
        }
    }

    public static CubeList BuildCubeList(CubeData cube, BitVectorFamily f0)
    {
        int total = f0.Count;
        var cubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(total, 1));
        int idx = 0;
        for (int si = 0; si < f0.Count; si++)
        {
            cubes[idx++] = f0.GetSet(si);
        }

        return new CubeList(cube.RentCofEmpty(), cubes, idx, rented: true, ownsCof: true, owner: cube);
    }

    public static CubeList BuildCubeList(CubeData cube, BitVectorFamily f0, BitVectorFamily f1)
    {
        int total = f0.Count + f1.Count;
        var cubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(total, 1));
        int idx = 0;
        for (int si = 0; si < f0.Count; si++)
        {
            cubes[idx++] = f0.GetSet(si);
        }

        for (int si = 0; si < f1.Count; si++)
        {
            cubes[idx++] = f1.GetSet(si);
        }

        return new CubeList(cube.RentCofEmpty(), cubes, idx, rented: true, ownsCof: true, owner: cube);
    }

    public static CubeList BuildCubeList(CubeData cube, BitVectorFamily f0, BitVectorFamily f1, BitVectorFamily f2)
    {
        int total = f0.Count + f1.Count + f2.Count;
        var cubes = ArrayPool<BitVector>.Shared.Rent(Math.Max(total, 1));
        int idx = 0;
        for (int si = 0; si < f0.Count; si++)
        {
            cubes[idx++] = f0.GetSet(si);
        }

        for (int si = 0; si < f1.Count; si++)
        {
            cubes[idx++] = f1.GetSet(si);
        }

        for (int si = 0; si < f2.Count; si++)
        {
            cubes[idx++] = f2.GetSet(si);
        }

        return new CubeList(cube.RentCofEmpty(), cubes, idx, rented: true, ownsCof: true, owner: cube);
    }
}
