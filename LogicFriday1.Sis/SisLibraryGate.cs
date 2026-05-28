using System.Text.Json.Serialization;

namespace LogicFriday1.Sis;

public sealed class SisLibraryGate
{
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    [JsonPropertyName("areaText")]
    public string AreaText { get; init; } = string.Empty;

    [JsonPropertyName("expression")]
    public string Expression { get; init; } = string.Empty;
}