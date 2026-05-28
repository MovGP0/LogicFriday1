using System.Numerics;

namespace Espresso;

public static class BitVectorOps
{
    public const int  Bpi      = 32;
    public const int  LogBpi   = 5;
    public const uint Disjoint = 0x55555555u;
    public static int WhichWord(int e) => e >> LogBpi;
    public static int WhichBit(int e) => e & (Bpi - 1);
    public static int WordCount(int size) => (size + Bpi - 1) >> LogBpi;
    public static void AddFlag(BitVector set, CubeFlags flag) { if (!set.IsNull) set.Meta |= (uint)flag; }
    public static void ClearFlag(BitVector set, CubeFlags flag) { if (!set.IsNull) set.Meta &= ~(uint)flag; }
    public static bool HasFlag(BitVector set, CubeFlags flag) =>
        !set.IsNull && (set.Meta & (uint)flag) != 0;
    public static CubeFlags GetFlags(BitVector set) =>
        set.IsNull ? CubeFlags.None : (CubeFlags)(byte)set.Meta;
    public static int GetSortKey(BitVector set) =>
        set.IsNull ? 0 : (int)(set.Meta >> 16);
    public static void SetSortKey(BitVector set, int size) { if (!set.IsNull) set.Meta = (set.Meta & 0xFFFFu) | ((uint)size << 16); }
    public static bool Contains(ReadOnlySpan<uint> set, int e) =>
        (set[e >> LogBpi] & (1u << (e & (Bpi - 1)))) != 0;
    public static void Remove(Span<uint> set, int e) =>
        set[e >> LogBpi] &= ~(1u << (e & (Bpi - 1)));
    public static void Insert(Span<uint> set, int e) =>
        set[e >> LogBpi] |= 1u << (e & (Bpi - 1));
    public static int CountOnes(uint v) => BitOperations.PopCount(v);
    public static void Copy(Span<uint> r, ReadOnlySpan<uint> a) => a.CopyTo(r);
    public static void Fill(Span<uint> r, int size)
    {
        r.Fill(~0u);
        r[^1] = ~0u >> (r.Length * Bpi - size);
    }
    public static void And(Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < r.Length; i++) r[i] = a[i] & b[i];
    }
    public static void Or(Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < r.Length; i++) r[i] = a[i] | b[i];
    }
    public static void AndNot(Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < r.Length; i++) r[i] = a[i] & ~b[i];
    }
    public static void Xor(Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < r.Length; i++) r[i] = a[i] ^ b[i];
    }
    public static void MergeWithMask(Span<uint> r, ReadOnlySpan<uint> a, ReadOnlySpan<uint> b, ReadOnlySpan<uint> mask)
    {
        for (int i = 0; i < r.Length; i++) r[i] = (a[i] & mask[i]) | (b[i] & ~mask[i]);
    }
    public static int BitIndex(uint a) =>
        a == 0 ? -1 : BitOperations.TrailingZeroCount(a);
    public static int PopCount(ReadOnlySpan<uint> a)
    {
        int sum = 0;
        for (int i = 0; i < a.Length; i++) if (a[i] != 0) sum += CountOnes(a[i]);
        return sum;
    }
    public static int IntersectionCount(ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        int sum = 0;
        for (int i = 0; i < a.Length; i++) { uint val = a[i] & b[i]; if (val != 0) sum += CountOnes(val); }
        return sum;
    }
    public static bool IsEmpty(ReadOnlySpan<uint> a) => !a.ContainsAnyExcept(0u);
    public static bool AreEqual(ReadOnlySpan<uint> a, ReadOnlySpan<uint> b) => a.SequenceEqual(b);
    public static bool AreDisjoint(ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < a.Length; i++)
            if ((a[i] & b[i]) != 0) return false;
        return true;
    }
    public static bool IsSubsetOf(ReadOnlySpan<uint> a, ReadOnlySpan<uint> b)
    {
        for (int i = 0; i < a.Length; i++)
            if ((a[i] & ~b[i]) != 0) return false;
        return true;
    }
    public static BitVector Create(int size)
    {
        int words = WordCount(size);
        return new BitVector(new uint[words + 1], 1, words);
    }
    public static BitVector Clone(BitVector r)
    {
        var p = new BitVector(new uint[r.Words + 1], 1, r.Words);
        r.AsSpan().CopyTo(p.AsSpan());
        return p;
    }
    public static int CompareDescending(BitVector a, BitVector b)
    {
        uint sa = (uint)GetSortKey(a), sb = (uint)GetSortKey(b);
        if (sa > sb) return -1;
        if (sa < sb) return 1;
        var spa = a.AsSpan(); var spb = b.AsSpan();
        for (int i = spa.Length - 1; i >= 0; i--)
        {
            if (spa[i] > spb[i]) return -1;
            if (spa[i] < spb[i]) return 1;
        }
        return 0;
    }
    public static int CompareAscending(BitVector a, BitVector b) => -CompareDescending(a, b);
}