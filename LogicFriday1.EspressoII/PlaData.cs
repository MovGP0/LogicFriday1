namespace Espresso;

public class PlaData
{
    public const int FType = 1, DType = 2, RType = 4, EqntottType = 16;

    public const int FdType = FType | DType, FrType = FType | RType, DrType = DType | RType, FdrType = FType | DType | RType;

    public BitVectorFamily? F, D, R;

    public CubeData Cube = CubeData.Empty;

    public int PlaType;

    public string?[]? Label;
}
