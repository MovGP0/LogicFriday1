using System.Reflection;
using System.Runtime.InteropServices;

namespace LogicFriday1.Sis.Interop;

internal static unsafe partial class NativeMethods
{
    private const string NativeLibraryName = "logicfriday1_sis";

    static NativeMethods()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveNativeLibrary);
    }

    private static nint ResolveNativeLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (!string.Equals(libraryName, NativeLibraryName, StringComparison.Ordinal))
        {
            return nint.Zero;
        }

        var assemblyDirectory = Path.GetDirectoryName(assembly.Location);
        if (string.IsNullOrEmpty(assemblyDirectory))
        {
            return nint.Zero;
        }

        var rid = GetRuntimeIdentifier();
        var fileName = GetNativeLibraryFileName();
        var path = Path.Combine(assemblyDirectory, "runtimes", rid, "native", fileName);

        return NativeLibrary.Load(path, assembly, searchPath);
    }

    private static string GetRuntimeIdentifier()
    {
        var os = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? "win"
            : RuntimeInformation.IsOSPlatform(OSPlatform.OSX)
                ? "osx"
                : RuntimeInformation.IsOSPlatform(OSPlatform.Linux)
                    ? "linux"
                    : throw new PlatformNotSupportedException("Unsupported operating system for LogicFriday1.Sis native library.");

        var architecture = RuntimeInformation.OSArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => throw new PlatformNotSupportedException("LogicFriday1.Sis supports x64 and ARM64 native libraries only."),
        };

        return $"{os}-{architecture}";
    }

    private static string GetNativeLibraryFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return $"{NativeLibraryName}.dll";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return $"lib{NativeLibraryName}.dylib";
        }

        return $"lib{NativeLibraryName}.so";
    }
}
