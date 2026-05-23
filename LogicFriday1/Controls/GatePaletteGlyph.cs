using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using LogicFriday1.Models;

namespace LogicFriday1.Controls;

public sealed class GatePaletteGlyph : Control
{
    public static readonly StyledProperty<GatePaletteItem?> ItemProperty =
        AvaloniaProperty.Register<GatePaletteGlyph, GatePaletteItem?>(nameof(Item));

    public GatePaletteItem? Item
    {
        get => GetValue(ItemProperty);
        set => SetValue(ItemProperty, value);
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);

        if (Item is not { } item)
        {
            return;
        }

        if (Bounds.Width <= 8 || Bounds.Height <= 8)
        {
            return;
        }

        var bounds = new Rect(0, 0, Bounds.Width, Bounds.Height).Deflate(4);
        if (bounds.Width <= 0 || bounds.Height <= 0)
        {
            return;
        }

        var pen = new Pen(FindBrush("LogicFriday.Brush.OnSurface", Brushes.Black), 1.4);
        var textBrush = FindBrush("LogicFriday.Brush.OnSurfaceVariant", Brushes.Black);

        if (item.Kind is GatePaletteKind.Submit or GatePaletteKind.Cancel or GatePaletteKind.Help)
        {
            DrawCenteredText(context, item.Label, bounds, textBrush, 11);
            return;
        }

        if (item.Kind == GatePaletteKind.Select)
        {
            DrawSelect(context, bounds, pen);
            return;
        }

        if (item.Kind == GatePaletteKind.Wire)
        {
            DrawWire(context, bounds, pen);
            return;
        }

        DrawGate(context, item, bounds, pen, textBrush);
    }

    private static void DrawGate(
        DrawingContext context,
        GatePaletteItem item,
        Rect bounds,
        Pen pen,
        IBrush textBrush)
    {
        const double sourceWidth = 100;
        const double sourceHeight = 56;

        var scale = Math.Min(bounds.Width / sourceWidth, bounds.Height / sourceHeight);
        if (scale <= 0)
        {
            return;
        }

        var origin = new Point(
            bounds.X + (bounds.Width - sourceWidth * scale) / 2,
            bounds.Y + (bounds.Height - sourceHeight * scale) / 2);

        Point P(double x, double y)
        {
            return new Point(origin.X + x * scale, origin.Y + y * scale);
        }

        GateSymbolRenderer.Draw(context, item.Kind, item.InputCount, string.Empty, P, pen, textBrush, scale);
    }

    private static void DrawSelect(DrawingContext context, Rect bounds, Pen pen)
    {
        var left = bounds.X + bounds.Width * 0.32;
        var top = bounds.Y + bounds.Height * 0.16;
        var points = new[]
        {
            new Point(left, top),
            new Point(left, top + bounds.Height * 0.62),
            new Point(left + bounds.Width * 0.16, top + bounds.Height * 0.48),
            new Point(left + bounds.Width * 0.27, top + bounds.Height * 0.75),
            new Point(left + bounds.Width * 0.38, top + bounds.Height * 0.7),
            new Point(left + bounds.Width * 0.27, top + bounds.Height * 0.45),
            new Point(left + bounds.Width * 0.48, top + bounds.Height * 0.45)
        };

        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(points[0], false);
            for (var index = 1; index < points.Length; index++)
            {
                stream.LineTo(points[index]);
            }

            stream.EndFigure(true);
        }

        context.DrawGeometry(null, pen, geometry);
    }

    private static void DrawWire(DrawingContext context, Rect bounds, Pen pen)
    {
        var y = bounds.Center.Y;
        context.DrawLine(pen, new Point(bounds.X + 4, y), new Point(bounds.Right - 4, y));
        context.DrawEllipse(null, pen, new Point(bounds.X + 4, y), 2.5, 2.5);
        context.DrawEllipse(null, pen, new Point(bounds.Right - 4, y), 2.5, 2.5);
    }

    private static void DrawCenteredText(DrawingContext context, string text, Rect bounds, IBrush brush, double fontSize)
    {
        var formattedText = GateSymbolRenderer.CreateText(text, brush, fontSize);
        var point = new Point(
            bounds.X + (bounds.Width - formattedText.Width) / 2,
            bounds.Y + (bounds.Height - formattedText.Height) / 2);

        context.DrawText(formattedText, point);
    }

    private static void DrawText(DrawingContext context, string text, Point point, IBrush brush, double fontSize)
    {
        GateSymbolRenderer.DrawText(context, text, point, brush, fontSize);
    }

    private IBrush FindBrush(string key, IBrush fallback)
    {
        if (Application.Current?.TryFindResource(key, out var resource) == true &&
            resource is IBrush brush)
        {
            return brush;
        }

        return fallback;
    }
}
