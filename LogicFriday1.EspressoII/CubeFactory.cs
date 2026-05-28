namespace Espresso;

public static class CubeFactory
{
    internal static CubeData Build(int numVars, int numBinaryVars, ReadOnlySpan<int> partSize, int cubeTemp = 10)
    {
        int size = 0;
        var firstPart = new int[numVars];
        var lastPart = new int[numVars];
        var ps = partSize.ToArray();
        for (int var = 0; var < numVars; var++)
        {
            if (var < numBinaryVars)
            {
                ps[var] = 2;
            }

            firstPart[var] = size;
            size += Math.Abs(ps[var]);
            lastPart[var] = size - 1;
        }

        var varMask = new BitVector[numVars];
        var binaryMask = BitVectorOps.Create(size);
        Span<uint> bmSpan = binaryMask.AsSpan();
        for (int var = 0; var < numVars; var++)
        {
            BitVector p = varMask[var] = BitVectorOps.Create(size);
            Span<uint> sp = p.AsSpan();
            for (int i = firstPart[var]; i <= lastPart[var]; i++)
            {
                BitVectorOps.Insert(sp, i);
            }

            if (var < numBinaryVars)
            {
                BitVectorOps.Or(bmSpan, bmSpan, sp);
            }
        }

        int inWord;
        uint inMask;
        if (numBinaryVars == 0)
        {
            inWord = -1;
            inMask = 0;
        }
        else
        {
            inWord = BitVectorOps.WhichWord(lastPart[numBinaryVars - 1]);
            inMask = bmSpan[inWord] & BitVectorOps.Disjoint;
        }

        var temp = new BitVector[cubeTemp];
        for (int i = 0; i < cubeTemp; i++)
        {
            temp[i] = BitVectorOps.Create(size);
        }

        var fullSet = BitVectorOps.Create(size);
        BitVectorOps.Fill(fullSet.AsSpan(), size);
        return new CubeData
        {
            Size = size,
            NumVars = numVars,
            NumBinaryVars = numBinaryVars,
            FirstPart = firstPart,
            LastPart = lastPart,
            PartSize = ps,
            VarMask = varMask,
            Temp = temp,
            FullSet = fullSet,
            EmptySet = BitVectorOps.Create(size),
            InWord = inWord,
            InMask = inMask,
        };
    }
}
