using LogicFriday1.Espresso.Interop;
using System.Runtime.InteropServices;

namespace LogicFriday1.Espresso;

public static class EspressoNative
{
    public static int AbiVersion()
    {
        return NativeMethods.logicfriday1_espresso_abi_version();
    }

    public static string MinimizePla(string input, EspressoMinimizeMode mode)
    {
        var result = NativeMethods.MinimizePla(input, (int)mode);
        try
        {
            if (result.Status == 0)
            {
                return result.Value == 0
                    ? throw new InvalidOperationException("Native Espresso returned a null result.")
                    : Marshal.PtrToStringUTF8(result.Value) ?? string.Empty;
            }

            var error = result.Error == 0
                ? "Native Espresso minimization failed."
                : Marshal.PtrToStringUTF8(result.Error);
            throw new InvalidOperationException(error);
        }
        finally
        {
            if (result.Value != 0)
            {
                NativeMethods.FreeString(result.Value);
            }

            if (result.Error != 0)
            {
                NativeMethods.FreeString(result.Error);
            }
        }
    }
}
