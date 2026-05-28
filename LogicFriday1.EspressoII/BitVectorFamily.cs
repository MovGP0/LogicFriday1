using System.Buffers;

namespace Espresso;

public class BitVectorFamily
{
    public uint[] Data = [];
    public int    Words;
    public int    Stride;
    public int    SfSize;
    public int    Capacity;
    public int    Count;
    public int    ActiveCount;
    public BitVector GetSet(int index) => new(Data, index * Stride + 1, Words);
    public Span<uint> GetSpan(int index) => Data.AsSpan(index * Stride + 1, Words);
    public void EnsureCapacity(int capacity) { Array.Resize(ref Data, capacity * Stride); }
    public static BitVectorFamily Create(int num, int size)
    {
        int words = BitVectorOps.WordCount(size);
        return new BitVectorFamily
        {
            SfSize = size, Words = words, Stride = words + 1,
            Capacity = num, Data = new uint[num * (words + 1)],
            Count = 0, ActiveCount = 0,
        };
    }
    public static BitVectorFamily Clone(BitVectorFamily a) =>
        CopyInto(Create(a.Count, a.SfSize), a);
    public static BitVectorFamily CopyInto(BitVectorFamily r, BitVectorFamily a)
    {
        r.SfSize = a.SfSize; r.Words = a.Words; r.Stride = a.Stride;
        r.Count = a.Count; r.ActiveCount = a.ActiveCount;
        Array.Copy(a.Data, 0, r.Data, 0, a.Stride * a.Count);
        return r;
    }
    public static BitVectorFamily Join(BitVectorFamily a, BitVectorFamily b)
    {
        if (a.SfSize != b.SfSize) throw new InvalidOperationException("sf_join: sf_size mismatch");
        int asize = a.Count * a.Stride;
        int bsize = b.Count * b.Stride;
        var r = Create(a.Count + b.Count, a.SfSize);
        r.Count       = a.Count + b.Count;
        r.ActiveCount = a.ActiveCount + b.ActiveCount;
        Array.Copy(a.Data, 0, r.Data, 0,     asize);
        Array.Copy(b.Data, 0, r.Data, asize, bsize);
        return r;
    }
    public static BitVectorFamily Append(BitVectorFamily a, BitVectorFamily b)
    {
        if (a.SfSize != b.SfSize) throw new InvalidOperationException("sf_append: sf_size mismatch");
        int asize = a.Count * a.Stride;
        int bsize = b.Count * b.Stride;
        int newCap = a.Count + b.Count;
        if (newCap > a.Capacity)
        {
            a.Capacity = newCap;
            a.EnsureCapacity(a.Capacity);
        }
        Array.Copy(b.Data, 0, a.Data, asize, bsize);
        a.Count       += b.Count;
        a.ActiveCount += b.ActiveCount;
        return a;
    }
    public static BitVectorFamily Add(BitVectorFamily a, BitVector s)
    {
        if (a.Count >= a.Capacity)
        {
            a.Capacity = a.Capacity + a.Capacity / 2 + 1;
            a.EnsureCapacity(a.Capacity);
        }
        s.CopyWithMetaTo(a.Data, a.Count * a.Stride, a.Stride);
        a.Count++;
        return a;
    }
    public static void SetAllFlags(BitVectorFamily a, CubeFlags flag)
    {
        uint mask = (uint)flag;
        for (int si = 0; si < a.Count; si++)
            a.Data[si * a.Stride] |= mask;
    }
    public static void ClearAllFlags(BitVectorFamily a, CubeFlags flag)
    {
        uint mask = (uint)flag;
        for (int si = 0; si < a.Count; si++)
            a.Data[si * a.Stride] &= ~mask;
    }
    public static void ActivateAll(BitVectorFamily a)
    {
        SetAllFlags(a, CubeFlags.Active);
        a.ActiveCount = a.Count;
    }
    public static BitVectorFamily CompactInactive(BitVectorFamily a)
    {
        int destIdx = 0, originalCount = a.Count, stride = a.Stride;
        for (int si = 0; si < originalCount; si++)
        {
            if ((a.Data[si * stride] & (uint)CubeFlags.Active) != 0)
            {
                if (destIdx != si) Array.Copy(a.Data, si * stride, a.Data, destIdx * stride, stride);
                destIdx++;
            }
            else a.Count--;
        }
        return a;
    }
    public static BitVectorFamily FromSortedArray(BitVector[] A1, int totcnt, int size)
    {
        var R = Create(totcnt, size);
        R.Count = totcnt;
        int stride = R.Stride;
        for (int i = 0; i < totcnt; i++)
            A1[i].CopyWithMetaTo(R.Data, i * stride, stride);
        return R;
    }
    public static BitVectorFamily FromSortedOrder(BitVectorFamily src, int[] order, int count)
    {
        var R = Create(count, src.SfSize);
        R.Count = count;
        int stride = R.Stride;
        for (int i = 0; i < count; i++)
            Array.Copy(src.Data, order[i] * stride, R.Data, i * stride, stride);
        return R;
    }
    public static BitVector[] ToSortedArray(BitVectorFamily A, Comparison<BitVector> compare)
    {
        var A1 = ArrayPool<BitVector>.Shared.Rent(Math.Max(A.Count, 1));
        for (int i = 0; i < A.Count; i++)
        {
            var p = A.GetSet(i);
            BitVectorOps.SetSortKey(p, BitVectorOps.PopCount(p.AsSpan()));
            A1[i] = p;
        }
        A1.AsSpan(0, A.Count).Sort(compare);
        return A1;
    }
    public static void ReturnSortedArray(BitVector[] A1) =>
        ArrayPool<BitVector>.Shared.Return(A1, clearArray: false);

    internal static int RmEqual(BitVector[] A1, int len, Comparison<BitVector> compare)
    {
        if (len == 0) return 0;
        int pdest = 0;
        for (int p = 1; p < len; p++)
            if (compare(A1[p], A1[p - 1]) != 0)
                A1[pdest++] = A1[p - 1];
        A1[pdest++] = A1[len - 1];
        return pdest;
    }
}