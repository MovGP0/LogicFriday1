namespace LogicFriday1.Sis;

public readonly record struct SisMapOptions(
    bool InvertOutputs = false,
    bool ReadLibraryNoDecomp = false,
    SisMapMode MapMode = SisMapMode.Default)
{
    internal uint ToNativeFlags()
    {
        var flags = 0U;
        if (InvertOutputs)
        {
            flags |= 1U;
        }

        if (ReadLibraryNoDecomp)
        {
            flags |= 2U;
        }

        if (MapMode == SisMapMode.M1)
        {
            flags |= 4U;
        }

        return flags;
    }
}