namespace Espresso;

public sealed record CubeData
{
    public static CubeData Empty { get; } = new()
    {
        Size = 0,
        NumVars = 0,
        NumBinaryVars = 0,
        FirstPart = [],
        LastPart = [],
        PartSize = [],
        VarMask = [],
        Temp = [],
        FullSet = BitVector.Null,
        EmptySet = BitVector.Null,
        InMask = 0,
        InWord = 0,
    };

    public required int Size { get; init; }
    public required int NumVars { get; init; }
    public required int NumBinaryVars { get; init; }
    public required int[] FirstPart { get; init; }
    public required int[] LastPart { get; init; }
    public required int[] PartSize { get; init; }
    public required BitVector[] VarMask { get; init; }
    public required BitVector[] Temp { get; init; }
    public required BitVector FullSet { get; init; }
    public required BitVector EmptySet { get; init; }
    public required uint InMask { get; init; }
    public required int InWord { get; init; }

    public List<uint[]> CofPool { get; } = [];
    public int NumMvVars => NumVars - NumBinaryVars;
    public BitVector RentCof()
    {
        int words = BitVectorOps.WordCount(Size);
        var pool = CofPool;
        uint[] arr;
        if (pool.Count > 0)
        {
            arr = pool[^1];
            pool.RemoveAt(pool.Count - 1);
        }
        else
        {
            arr = new uint[words + 1];
        }

        arr[0] = 0;
        return new BitVector(arr, 1, words);
    }

    public BitVector RentCofEmpty()
    {
        int words = BitVectorOps.WordCount(Size);
        var pool = CofPool;
        uint[] arr;
        if (pool.Count > 0)
        {
            arr = pool[^1];
            pool.RemoveAt(pool.Count - 1);
            Array.Clear(arr, 0, words + 1);
        }
        else
        {
            arr = new uint[words + 1];
        }

        return new BitVector(arr, 1, words);
    }

    public void ReturnCof(BitVector b)
    {
        if (b.RawData != null)
        {
            CofPool.Add(b.RawData);
        }
    }

    public BitVector RentCofCopy(BitVector src)
    {
        int words = BitVectorOps.WordCount(Size);
        var pool = CofPool;
        uint[] arr;
        if (pool.Count > 0)
        {
            arr = pool[^1];
            pool.RemoveAt(pool.Count - 1);
        }
        else
        {
            arr = new uint[words + 1];
        }

        arr[0] = 0;
        src.AsSpan().CopyTo(arr.AsSpan(1, words));
        return new BitVector(arr, 1, words);
    }

    public int Output => NumMvVars > 0 ? NumVars - 1 : -1;

    public int FirstWordOf(int var) => BitVectorOps.WhichWord(FirstPart[var]);

    public int LastWordOf(int var) => BitVectorOps.WhichWord(LastPart[var]);

    public bool IsSparse(int var) => var >= NumBinaryVars;
}
