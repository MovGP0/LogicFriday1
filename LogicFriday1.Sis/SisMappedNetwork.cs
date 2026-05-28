using System.Text.Json.Serialization;

namespace LogicFriday1.Sis;

public sealed class SisMappedNetwork
{
    [JsonPropertyName("inputs")]
    public IReadOnlyList<string> Inputs { get; init; } = [];

    [JsonPropertyName("outputs")]
    public IReadOnlyList<string> Outputs { get; init; } = [];

    [JsonPropertyName("libraryGateCount")]
    public int LibraryGateCount { get; init; }

    [JsonPropertyName("libraryGates")]
    public IReadOnlyList<SisLibraryGate> LibraryGates { get; init; } = [];

    [JsonPropertyName("readLibraryNoDecomp")]
    public bool ReadLibraryNoDecomp { get; init; }

    [JsonPropertyName("mapMode")]
    public string MapMode { get; init; } = string.Empty;

    [JsonPropertyName("printGate")]
    public string PrintGate { get; init; } = string.Empty;

    [JsonPropertyName("printLevelSummary")]
    public string PrintLevelSummary { get; init; } = string.Empty;

    [JsonPropertyName("printLevel")]
    public string PrintLevel { get; init; } = string.Empty;

    [JsonPropertyName("gates")]
    public IReadOnlyList<SisMappedGate> Gates { get; init; } = [];
}