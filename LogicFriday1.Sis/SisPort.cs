using System.Text;
using System.Text.Json;
using LogicFriday1.Sis.Interop;

namespace LogicFriday1.Sis;

/// <summary>
/// Managed entry point for the incremental Rust SIS port.
/// </summary>
public static class SisPort
{
    private static readonly JsonSerializerOptions s_jsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    /// <summary>
    /// Gets the SIS interop design marker version.
    /// </summary>
    public static int AbiVersion => NativeMethods.logicfriday1_sis_abi_version();

    /// <summary>
    /// Maps a BLIF-style two-level function into a gate-network payload.
    /// </summary>
    public static SisMappedNetwork MapBlifToGates(string blif, SisMapOptions options = default) => MapBlifToGates(blif, genlib: null, options);

    /// <summary>
    /// Maps a BLIF-style two-level function and optional genlib text into a gate-network payload.
    /// </summary>
    public static unsafe SisMappedNetwork MapBlifToGates(string blif, string? genlib, SisMapOptions options = default)
    {
        ArgumentNullException.ThrowIfNull(blif);

        var input = Encoding.UTF8.GetBytes(blif);
        var library = genlib is null ? [] : Encoding.UTF8.GetBytes(genlib);
        fixed (byte* inputPointer = input)
        {
            fixed (byte* libraryPointer = library)
            {
                var required = NativeMethods.logicfriday1_sis_map_blif_genlib_to_json(
                    inputPointer,
                    (nuint)input.Length,
                    libraryPointer,
                    (nuint)library.Length,
                    options.ToNativeFlags(),
                    null,
                    0);

                if (required == 0)
                {
                    throw new SisMappingException(GetLastError());
                }

                var output = new byte[(int)required + 1];
                fixed (byte* outputPointer = output)
                {
                    var written = NativeMethods.logicfriday1_sis_map_blif_genlib_to_json(
                        inputPointer,
                        (nuint)input.Length,
                        libraryPointer,
                        (nuint)library.Length,
                        options.ToNativeFlags(),
                        outputPointer,
                        (nuint)output.Length);

                    if (written == 0)
                    {
                        throw new SisMappingException(GetLastError());
                    }

                    var json = Encoding.UTF8.GetString(output, 0, (int)written);
                    return JsonSerializer.Deserialize<SisMappedNetwork>(json, s_jsonOptions)
                        ?? throw new SisMappingException("Native SIS mapper returned an empty mapping payload.");
                }
            }
        }
    }

    private static unsafe string GetLastError()
    {
        var required = NativeMethods.logicfriday1_sis_last_error(null, 0);
        if (required == 0)
        {
            return "Native SIS mapper failed without an error message.";
        }

        var output = new byte[(int)required + 1];
        fixed (byte* outputPointer = output)
        {
            var written = NativeMethods.logicfriday1_sis_last_error(outputPointer, (nuint)output.Length);
            return Encoding.UTF8.GetString(output, 0, (int)written);
        }
    }
}
