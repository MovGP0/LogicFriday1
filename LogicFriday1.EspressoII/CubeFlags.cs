namespace Espresso;

[Flags]
public enum CubeFlags : byte
{
    None = 0,

    Prime = 0x01,

    NonEssen = 0x02,

    Active = 0x04,

    Redund = 0x08,

    Covered = 0x10,

    RelEssen = 0x20,
}
