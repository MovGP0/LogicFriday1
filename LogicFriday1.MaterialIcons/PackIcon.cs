using System;
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
        Width = 18;
        Height = 18;
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

    public override void Render(DrawingContext context)
    {
        if (Data is not { } data ||
            Bounds.Width <= 0 ||
            Bounds.Height <= 0)
        {
            return;
        }

        var sourceBounds = data.Bounds;
        if (sourceBounds.Width <= 0 ||
            sourceBounds.Height <= 0)
        {
            return;
        }

        var scale = Math.Min(Bounds.Width / sourceBounds.Width, Bounds.Height / sourceBounds.Height);
        var x = (Bounds.Width - sourceBounds.Width * scale) / 2;
        var y = (Bounds.Height - sourceBounds.Height * scale) / 2;
        var matrix =
            Matrix.CreateTranslation(-sourceBounds.X, -sourceBounds.Y) *
            Matrix.CreateScale(scale, scale) *
            Matrix.CreateTranslation(x, y);

        var geometry = data.Clone();
        geometry.Transform = new MatrixTransform(matrix);

        context.DrawGeometry(Foreground ?? Brushes.Black, null, geometry);
    }
}
