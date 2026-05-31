using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace LogicFriday1.Espresso.Interop;

internal static partial class NativeMethods
{
    private const string NativeLibraryName = "logicfriday1_espresso";

    [StructLayout(LayoutKind.Sequential)]
    internal readonly struct StringResult
    {
        public readonly int Status;

        public readonly nint Value;

        public readonly nint Error;
    }

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
                    : throw new PlatformNotSupportedException("Unsupported operating system for LogicFriday1.Espresso native library.");

        var architecture = RuntimeInformation.OSArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => throw new PlatformNotSupportedException("LogicFriday1.Espresso supports x64 and ARM64 native libraries only."),
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

    [DllImport(NativeLibraryName, EntryPoint = "logicfriday1_espresso_minimize_pla", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern StringResult MinimizePla(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        int mode);

    [DllImport(NativeLibraryName, EntryPoint = "logicfriday1_espresso_string_free", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void FreeString(nint value);
}
