namespace Espresso;

public readonly struct BitVector(
    uint[] data,
    int offset,
    int words)
{
    public bool IsNull => data is null;

    public static BitVector Null => default;

    public int Words => words;

    internal uint[] RawData => data;

    public Span<uint> AsSpan() => data.AsSpan(offset, words);

    public ref uint Meta => ref data[offset - 1];

    public void CopyWithMetaTo(uint[] destArray, int destOffset, int stride) =>
        Array.Copy(data, offset - 1, destArray, destOffset, stride);

    public static bool Overlaps(ReadOnlySpan<uint> a, ReadOnlySpan<uint> b) => a.Overlaps(b);
}
