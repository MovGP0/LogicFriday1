using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Espresso;

/// <summary>
/// Streaming 128-bit hash using MurmurHash3 x64 128-bit algorithm.
/// </summary>
internal struct Hash128()
{
    private List<byte> _bytes = [];

    public ulong Hi = 0;

    public ulong Lo = 0;

    public void MixU64(ulong v)
    {
        Span<byte> buf = stackalloc byte[8];
        BinaryPrimitives.WriteUInt64LittleEndian(buf, v);
        _bytes.AddRange(buf);
    }

    public void MixSpan(ReadOnlySpan<uint> s)
    {
        // Mix length, then the raw bytes of the span.
        MixU64((ulong)s.Length);
        _bytes.AddRange(MemoryMarshal.AsBytes(s));
    }

    public void Finalize128()
    {
        var span = CollectionsMarshal.AsSpan(_bytes);
        (Lo, Hi) = Murmur3_x64_128(span);
    }

    public static Hash128 Of(ReadOnlySpan<uint> s)
    {
        var h = new Hash128();
        h.MixSpan(s);
        h.Finalize128();
        return h;
    }

    private static (ulong h1, ulong h2) Murmur3_x64_128(ReadOnlySpan<byte> data)
    {
        const ulong C1 = 0x87C37B91114253D5UL;
        const ulong C2 = 0x4CF5AD432745937FUL;
        ulong h1 = 0, h2 = 0;
        ulong len = (ulong)data.Length;
        int nblocks = data.Length / 16;
        for (int i = 0; i < nblocks; i++)
        {
            ulong k1 = BinaryPrimitives.ReadUInt64LittleEndian(data.Slice(i * 16));
            ulong k2 = BinaryPrimitives.ReadUInt64LittleEndian(data.Slice(i * 16 + 8));
            k1 *= C1;
            k1 = RotL64(k1, 31);
            k1 *= C2;
            h1 ^= k1;
            h1 = RotL64(h1, 27);
            h1 += h2;
            h1 = h1 * 5 + 0x52DCE729UL;
            k2 *= C2;
            k2 = RotL64(k2, 33);
            k2 *= C1;
            h2 ^= k2;
            h2 = RotL64(h2, 31);
            h2 += h1;
            h2 = h2 * 5 + 0x38495AB5UL;
        }

        var tail = data.Slice(nblocks * 16);
        ulong tk1 = 0, tk2 = 0;
        switch (tail.Length)
        {
            case 15:
                tk2 ^= (ulong)tail[14] << 48;
                goto case 14;
            case 14:
                tk2 ^= (ulong)tail[13] << 40;
                goto case 13;
            case 13:
                tk2 ^= (ulong)tail[12] << 32;
                goto case 12;
            case 12:
                tk2 ^= (ulong)tail[11] << 24;
                goto case 11;
            case 11:
                tk2 ^= (ulong)tail[10] << 16;
                goto case 10;
            case 10:
                tk2 ^= (ulong)tail[9] << 8;
                goto case 9;
            case 9:
                tk2 ^= tail[8];
                tk2 *= C2;
                tk2 = RotL64(tk2, 33);
                tk2 *= C1;
                h2 ^= tk2;
                goto case 8;
            case 8:
                tk1 ^= (ulong)tail[7] << 56;
                goto case 7;
            case 7:
                tk1 ^= (ulong)tail[6] << 48;
                goto case 6;
            case 6:
                tk1 ^= (ulong)tail[5] << 40;
                goto case 5;
            case 5:
                tk1 ^= (ulong)tail[4] << 32;
                goto case 4;
            case 4:
                tk1 ^= (ulong)tail[3] << 24;
                goto case 3;
            case 3:
                tk1 ^= (ulong)tail[2] << 16;
                goto case 2;
            case 2:
                tk1 ^= (ulong)tail[1] << 8;
                goto case 1;
            case 1:
                tk1 ^= tail[0];
                tk1 *= C1;
                tk1 = RotL64(tk1, 31);
                tk1 *= C2;
                h1 ^= tk1;
                break;
        }

        h1 ^= len;
        h2 ^= len;
        h1 += h2;
        h2 += h1;
        h1 = FMix64(h1);
        h2 = FMix64(h2);
        h1 += h2;
        h2 += h1;
        return (h1, h2);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static ulong RotL64(ulong x, int r) => (x << r) | (x >> (64 - r));

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static ulong FMix64(ulong k)
    {
        k ^= k >> 33;
        k *= 0xFF51AFD7ED558CCDUL;
        k ^= k >> 33;
        k *= 0xC4CEB9FE1A85EC53UL;
        k ^= k >> 33;
        return k;
    }
}
