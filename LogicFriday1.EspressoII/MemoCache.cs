using System.Runtime.CompilerServices;

namespace Espresso;

/// <summary>
/// Keyed on a 128-bit hash of (schema, operation tag, serialized inputs).
/// Always enabled; persists to %APPDATA%/Espresso/cache.bin by default.
///
/// Override via env var:
///   ESPRESSO_CACHE_FILE=&lt;path&gt;   — use a different cache file path
///
/// Correctness note: keys are 128 bits; the probability of a false positive over
/// 10^9 entries is ~3e-21. We do not store the original inputs for verification,
/// so a collision would silently produce a wrong result — the 128-bit width
/// keeps this statistically negligible.
/// </summary>
public static class MemoCache
{
    public const byte TagIsTautology = 1;

    public const byte TagIsCubeCovered = 2;

    public const byte TagMinimize = 3;

    public const byte TagFindIrredundant = 4;

    public const byte TagExpandCover = 5;

    public const byte TagMakeSparse = 6;

    public const byte TagComplement = 7;

    private const uint Magic = 0x4D50534Eu;

    private const uint Version = 1;

    public readonly record struct Key(ulong Hi, ulong Lo);

    private static readonly Dictionary<Key, byte[]> _store = new(capacity: 1 << 14);

    private static readonly object _sync = new();

    private static string? _persistPath;

    private static bool _enabled = true;

    private static bool _dirty;

    private static long _hits, _misses, _puts;

    private static readonly ConditionalWeakTable<CubeData, object> _schemaHashes = new();

    public static bool Enabled => _enabled;

    public static long Hits => Interlocked.Read(ref _hits);

    public static long Misses => Interlocked.Read(ref _misses);

    public static long Puts => Interlocked.Read(ref _puts);

    public static int EntryCount
    {
        get
        {
            lock (_sync)
            {
                return _store.Count;
            }
        }
    }

    public static void Init()
    {
        string? envPath = Environment.GetEnvironmentVariable("ESPRESSO_CACHE_FILE");
        _persistPath = !string.IsNullOrEmpty(envPath)
            ? envPath
            : Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "Espresso",
                "cache.bin");
        Directory.CreateDirectory(Path.GetDirectoryName(_persistPath)!);
        TryLoad(_persistPath);
        AppDomain.CurrentDomain.ProcessExit += (_, _) => TrySave();
    }

    public static void Flush() => TrySave();

    public static void Clear()
    {
        lock (_sync)
        {
            _store.Clear();
            _dirty = true;
        }
    }

    public static ulong SchemaHash(CubeData cube)
    {
        if (_schemaHashes.TryGetValue(cube, out var boxed))
        {
            return (ulong)boxed!;
        }

        var h = new Hash128();
        h.MixU64((ulong)cube.Size);
        h.MixU64((ulong)cube.NumVars);
        h.MixU64((ulong)cube.NumBinaryVars);
        h.MixU64((ulong)cube.InWord);
        h.MixU64(cube.InMask);
        foreach (int p in cube.PartSize)
        {
            h.MixU64((ulong)p);
        }

        ulong hashed = h.Hi ^ h.Lo;
        _schemaHashes.Add(cube, hashed);
        return hashed;
    }

    public static bool TryGet(in Key key, out byte[] value)
    {
        if (!_enabled)
        {
            value = null!;
            return false;
        }

        lock (_sync)
        {
            if (_store.TryGetValue(key, out var got))
            {
                Interlocked.Increment(ref _hits);
                value = got;
                return true;
            }
        }

        Interlocked.Increment(ref _misses);
        value = null!;
        return false;
    }

    public static void Put(in Key key, byte[] value)
    {
        if (!_enabled)
        {
            return;
        }

        lock (_sync)
        {
            _store[key] = value;
            _dirty = true;
        }

        Interlocked.Increment(ref _puts);
    }

    public static bool TryGetBool(in Key key, out bool value)
    {
        if (TryGet(key, out var bytes) && bytes.Length == 1)
        {
            value = bytes[0] != 0;
            return true;
        }

        value = false;
        return false;
    }

    public static void PutBool(in Key key, bool value) => Put(key, [value ? (byte)1 : (byte)0]);

    // ---- Key building ----

    public static Key BuildCubeListKey(byte tag, CubeData cube, CubeList T)
    {
        var h = new Hash128();
        h.MixU64(SchemaHash(cube));
        h.MixU64(tag);
        h.MixU64((ulong)T.Count);
        h.MixSpan(T.CofSpan);
        for (int i = 0; i < T.Count; i++)
        {
            h.MixSpan(T.GetSpan(i));
        }

        h.Finalize128();
        return new Key(h.Hi, h.Lo);
    }

    public static Key BuildCubeListCubeKey(byte tag, CubeData cube, CubeList T, ReadOnlySpan<uint> c)
    {
        var h = new Hash128();
        h.MixU64(SchemaHash(cube));
        h.MixU64(tag);
        h.MixU64((ulong)T.Count);
        h.MixSpan(T.CofSpan);
        h.MixSpan(c);
        for (int i = 0; i < T.Count; i++)
        {
            h.MixSpan(T.GetSpan(i));
        }

        h.Finalize128();
        return new Key(h.Hi, h.Lo);
    }

    public static Key BuildMinimizeKey(CubeData cube, BitVectorFamily F, BitVectorFamily D, BitVectorFamily R)
        => BuildFamiliesKey(TagMinimize, cube, F, D, R, extra: 0);

    public static Key BuildFamiliesKey(byte tag, CubeData cube, BitVectorFamily A, BitVectorFamily? B, BitVectorFamily? C, long extra)
    {
        var h = new Hash128();
        h.MixU64(SchemaHash(cube));
        h.MixU64(tag);
        h.MixU64((ulong)extra);
        MixFamily(ref h, A);
        if (B is not null)
        {
            MixFamily(ref h, B);
        }

        if (C is not null)
        {
            MixFamily(ref h, C);
        }

        h.Finalize128();
        return new Key(h.Hi, h.Lo);
    }

    public static Key BuildCubeListFamilyKey(byte tag, CubeData cube, CubeList T)
    {
        var h = new Hash128();
        h.MixU64(SchemaHash(cube));
        h.MixU64(tag);
        h.MixU64((ulong)T.Count);
        h.MixSpan(T.CofSpan);
        // Row-order-invariant across T rows.
        ulong accHi = 0, accLo = 0, sumHi = 0, sumLo = 0;
        for (int i = 0; i < T.Count; i++)
        {
            var rh = Hash128.Of(T.GetSpan(i));
            accHi ^= rh.Hi;
            accLo ^= rh.Lo;
            sumHi += rh.Hi;
            sumLo += rh.Lo;
        }

        h.MixU64(accHi);
        h.MixU64(accLo);
        h.MixU64(sumHi);
        h.MixU64(sumLo);
        h.Finalize128();
        return new Key(h.Hi, h.Lo);
    }

    private static void MixFamily(ref Hash128 h, BitVectorFamily fam)
    {
        h.MixU64((ulong)fam.SfSize);
        h.MixU64((ulong)fam.Count);
        int words = fam.Words;
        // Row-order-invariant: combine per-row 128-bit hashes via commutative XOR + ADD.
        ulong accHi = 0, accLo = 0;
        ulong sumHi = 0, sumLo = 0;
        for (int i = 0; i < fam.Count; i++)
        {
            var rh = Hash128.Of(fam.GetSpan(i));
            accHi ^= rh.Hi;
            accLo ^= rh.Lo;
            sumHi += rh.Hi;
            sumLo += rh.Lo;
        }

        h.MixU64(accHi);
        h.MixU64(accLo);
        h.MixU64(sumHi);
        h.MixU64(sumLo);
    }

    public static bool TryGetFamily(in Key key, int expectedSize, out BitVectorFamily fam)
    {
        fam = null!;
        if (!TryGet(key, out var bytes))
        {
            return false;
        }

        if (bytes.Length < 8)
        {
            return false;
        }

        int sfSize = BitConverter.ToInt32(bytes, 0);
        int count = BitConverter.ToInt32(bytes, 4);
        if (sfSize != expectedSize)
        {
            return false;
        }

        int words = (sfSize + 31) >> 5;
        int stride = words + 1;
        int expectedBytes = 8 + count * stride * 4;
        if (bytes.Length != expectedBytes)
        {
            return false;
        }

        fam = BitVectorFamily.Create(Math.Max(count, 1), sfSize);
        fam.Count = count;
        fam.ActiveCount = 0;
        Buffer.BlockCopy(bytes, 8, fam.Data, 0, count * stride * 4);
        return true;
    }

    public static void PutFamily(in Key key, BitVectorFamily fam)
    {
        if (!_enabled)
        {
            return;
        }

        int stride = fam.Stride;
        int payload = fam.Count * stride * 4;
        byte[] bytes = new byte[8 + payload];
        BitConverter.GetBytes(fam.SfSize).CopyTo(bytes, 0);
        BitConverter.GetBytes(fam.Count).CopyTo(bytes, 4);
        Buffer.BlockCopy(fam.Data, 0, bytes, 8, payload);
        Put(key, bytes);
    }

    // ---- Disk persistence ----

    private static void TryLoad(string path)
    {
        try
        {
            if (!File.Exists(path))
            {
                return;
            }

            using var fs = File.OpenRead(path);
            using var br = new BinaryReader(fs);
            if (br.ReadUInt32() != Magic)
            {
                return;
            }

            if (br.ReadUInt32() != Version)
            {
                return;
            }

            int count = br.ReadInt32();
            lock (_sync)
            {
                _store.EnsureCapacity(count);
                for (int i = 0; i < count; i++)
                {
                    ulong hi = br.ReadUInt64();
                    ulong lo = br.ReadUInt64();
                    int len = br.ReadInt32();
                    var data = br.ReadBytes(len);
                    _store[new Key(hi, lo)] = data;
                }
            }
        }
        catch { /* corrupt/partial cache → ignore */ }
    }

    private static void TrySave()
    {
        if (_persistPath is null)
        {
            return;
        }

        if (!_dirty)
        {
            return;
        }

        try
        {
            string tmp = _persistPath + ".tmp";
            using (var fs = File.Create(tmp))
            using (var bw = new BinaryWriter(fs))
            {
                bw.Write(Magic);
                bw.Write(Version);
                KeyValuePair<Key, byte[]>[] snapshot;
                lock (_sync)
                {
                    snapshot = _store.ToArray();
                }

                bw.Write(snapshot.Length);
                foreach (var kv in snapshot)
                {
                    bw.Write(kv.Key.Hi);
                    bw.Write(kv.Key.Lo);
                    bw.Write(kv.Value.Length);
                    bw.Write(kv.Value);
                }
            }
            File.Move(tmp, _persistPath!, overwrite: true);
            _dirty = false;
        }
        catch { /* non-fatal */ }
    }
}
