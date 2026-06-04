namespace LogicFriday1.Services;

public sealed record MinimizeOptions(
    bool UseExactMode,
    bool MinimizeOutputsIndependently)
{
    public static MinimizeOptions Default { get; } = new(
        UseExactMode: false,
        MinimizeOutputsIndependently: true);
}
