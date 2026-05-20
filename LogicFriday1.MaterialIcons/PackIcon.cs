using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;

namespace LogicFriday1.MaterialIcons;

public class PackIcon : PathIcon
{
    public static readonly StyledProperty<PackIconKind> KindProperty =
        AvaloniaProperty.Register<PackIcon, PackIconKind>(nameof(Kind));

    static PackIcon()
    {
        KindProperty.Changed.AddClassHandler<PackIcon>((icon, _) => icon.UpdateData());
    }

    public PackIcon()
    {
        Width = 16;
        Height = 16;
        UpdateData();
    }

    public PackIconKind Kind
    {
        get => GetValue(KindProperty);
        set => SetValue(KindProperty, value);
    }

    private void UpdateData()
    {
        var data = PackIconDataFactory.GetData(Kind);
        Data = string.IsNullOrEmpty(data)
            ? null
            : StreamGeometry.Parse(data);
    }
}
