namespace Espresso;

public static class GaspOptimizer
{
    internal static bool IsFeasiblyCovered(CubeData cube, BitVectorFamily R, BitVector p, BitVector RAISE)
    {
        Span<uint> sr = cube.Temp[0].AsSpan();
        BitVectorOps.Or(sr, RAISE.AsSpan(), p.AsSpan());
        for (int i = 0; i < R.Count; i++)
        {
            if (!BitVectorOps.HasFlag(R.GetSet(i), CubeFlags.Active))
            {
                continue;
            }

            if (CubeDistance.DistanceCapped(cube, R.GetSpan(i), sr) == 0)
            {
                return false;
            }
        }

        return true;
    }
}
