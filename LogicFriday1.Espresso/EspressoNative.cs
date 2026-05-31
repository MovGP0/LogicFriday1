using LogicFriday1.Espresso.Interop;

namespace LogicFriday1.Espresso;

public static class EspressoNative
{
    public static int AbiVersion()
    {
        return NativeMethods.logicfriday1_espresso_abi_version();
    }
}
