namespace Espresso;

public static class CubeDistance
{
    public static bool IsFullCoverage(CubeData cube, ReadOnlySpan<uint> p, ReadOnlySpan<uint> cof)
    {
        var sf = cube.FullSet.AsSpan();
        for (int i = p.Length - 1; i >= 0; i--)
        {
            if ((p[i] | cof[i]) != sf[i])
            {
                return false;
            }
        }

        return true;
    }

    private static int CountBinaryDisjoint(CubeData cube, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        int last = cube.InWord;
        if (last == -1)
        {
            return 0;
        }

        uint x = a[last] & b[last];
        x = ~(x | (x >> 1)) & cube.InMask;
        int dist = x != 0 ? BitVectorOps.CountOnes(x) : 0;
        for (int w = 0; w < last; w++)
        {
            x = a[w] & b[w];
            x = ~(x | (x >> 1)) & BitVectorOps.Disjoint;
            if (x != 0)
            {
                dist += BitVectorOps.CountOnes(x);
            }
        }

        return dist;
    }

    private static bool IsMvVariableDisjoint(CubeData cube, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b, int var)
    {
        ReadOnlySpan<uint> sm = cube.VarMask[var].AsSpan();
        int last = cube.LastWordOf(var);
        for (int w = cube.FirstWordOf(var); w <= last; w++)
        {
            if ((a[w] & b[w] & sm[w]) != 0)
            {
                return false;
            }
        }

        return true;
    }

    public static bool AreDistance0(CubeData cube, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        if (CountBinaryDisjoint(cube, a, b) != 0)
        {
            return false;
        }

        for (int var = cube.NumBinaryVars; var < cube.NumVars; var++)
        {
            if (IsMvVariableDisjoint(cube, a, b, var))
            {
                return false;
            }
        }

        return true;
    }

    public static int DistanceCapped(CubeData cube, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        int dist = 0, last = cube.InWord;
        if (last != -1)
        {
            uint x = a[last] & b[last];
            x = ~(x | (x >> 1)) & cube.InMask;
            if (x != 0)
            {
                dist = BitVectorOps.CountOnes(x);
                if (dist > 1)
                {
                    return 2;
                }
            }

            for (int w = 0; w < last; w++)
            {
                x = a[w] & b[w];
                x = ~(x | (x >> 1)) & BitVectorOps.Disjoint;
                if (x != 0 && (dist == 1 || (dist += BitVectorOps.CountOnes(x)) > 1))
                {
                    return 2;
                }
            }
        }

        for (int var = cube.NumBinaryVars; var < cube.NumVars; var++)
        {
            if (IsMvVariableDisjoint(cube, a, b, var) && ++dist > 1)
            {
                return 2;
            }
        }

        return dist;
    }

    public static int Distance(CubeData cube, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        int dist = CountBinaryDisjoint(cube, a, b);
        for (int var = cube.NumBinaryVars; var < cube.NumVars; var++)
        {
            if (IsMvVariableDisjoint(cube, a, b, var))
            {
                dist++;
            }
        }

        return dist;
    }

    public static void FindDisjointParts(CubeData cube, Span<uint> xlower, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        int last = cube.InWord;
        if (last != -1)
        {
            uint x = a[last] & b[last];
            x = ~(x | (x >> 1)) & cube.InMask;
            if (x != 0)
            {
                xlower[last] |= (x | (x << 1)) & a[last];
            }

            for (int w = 0; w < last; w++)
            {
                x = a[w] & b[w];
                x = ~(x | (x >> 1)) & BitVectorOps.Disjoint;
                if (x != 0)
                {
                    xlower[w] |= (x | (x << 1)) & a[w];
                }
            }
        }

        for (int var = cube.NumBinaryVars; var < cube.NumVars; var++)
        {
            if (IsMvVariableDisjoint(cube, a, b, var))
            {
                ReadOnlySpan<uint> sm = cube.VarMask[var].AsSpan();
                for (int w = cube.FirstWordOf(var); w <= cube.LastWordOf(var); w++)
                {
                    xlower[w] |= a[w] & sm[w];
                }
            }
        }
    }

    public static void Consensus(CubeData cube, Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        r.Clear();
        int last = cube.InWord;
        if (last != -1)
        {
            uint x = a[last] & b[last];
            r[last] = x;
            x = ~(x | (x >> 1)) & cube.InMask;
            if (x != 0)
            {
                r[last] |= (x | (x << 1)) & (a[last] | b[last]);
            }

            for (int w = 0; w < last; w++)
            {
                x = a[w] & b[w];
                r[w] = x;
                x = ~(x | (x >> 1)) & BitVectorOps.Disjoint;
                if (x != 0)
                {
                    r[w] |= (x | (x << 1)) & (a[w] | b[w]);
                }
            }
        }

        for (int var = cube.NumBinaryVars; var < cube.NumVars; var++)
        {
            ReadOnlySpan<uint> sm = cube.VarMask[var].AsSpan();
            int mvLast = cube.LastWordOf(var);
            if (IsMvVariableDisjoint(cube, a, b, var))
            {
                for (int w = cube.FirstWordOf(var); w <= mvLast; w++)
                {
                    r[w] |= sm[w] & (a[w] | b[w]);
                }
            }
            else
            {
                for (int w = cube.FirstWordOf(var); w <= mvLast; w++)
                {
                    r[w] |= a[w] & b[w] & sm[w];
                }
            }
        }
    }

    public static int Distance1Order(CubeData cube, BitVector a, BitVector b)
    {
        ReadOnlySpan<uint> sa = a.AsSpan(), sb = b.AsSpan(), sc = cube.Temp[0].AsSpan();
        for (int i = sa.Length - 1; i >= 0; i--)
        {
            uint x1 = sa[i] | sc[i], x2 = sb[i] | sc[i];
            if (x1 > x2)
            {
                return -1;
            }

            if (x1 < x2)
            {
                return 1;
            }
        }

        return 0;
    }
}
