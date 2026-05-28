using System.Buffers;

namespace Espresso;

public readonly struct CubeList(
    BitVector cof,
    BitVector[] cubes,
    int count,
    bool rented = false,
    bool ownsCof = false,
    CubeData? owner = null)
{
    public readonly BitVector Cof = cof;
    public readonly BitVector[] Cubes = cubes;
    public readonly int Count = count;
    public readonly bool Rented = rented;
    public readonly bool OwnsCof = ownsCof;
    public readonly CubeData Owner = owner!;
    public BitVector this[int i] => Cubes[i];
    public Span<uint> GetSpan(int i) => Cubes[i].AsSpan();
    public ReadOnlySpan<uint> CofSpan => Cof.AsSpan();

    public void ReturnCubes()
    {
        if (Rented && Cubes != null) ArrayPool<BitVector>.Shared.Return(Cubes, clearArray: false);
        if (OwnsCof && Owner != null) Owner.ReturnCof(Cof);
    }
}