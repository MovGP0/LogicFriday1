using System.Globalization;
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

        switch (item.Kind)
        {
            case GatePaletteKind.Not:
                DrawNot(context, P, scale, pen);
                break;

            case GatePaletteKind.Nand:
            case GatePaletteKind.And:
                DrawAnd(context, item.Kind == GatePaletteKind.Nand, item.InputCount, P, scale, pen);
                break;

            case GatePaletteKind.Nor:
            case GatePaletteKind.Or:
            case GatePaletteKind.Xor:
                DrawOr(context, item.Kind == GatePaletteKind.Nor, item.Kind == GatePaletteKind.Xor, item.InputCount, P, scale, pen);
                break;

            case GatePaletteKind.Mux:
                DrawMux(context, P, scale, pen, textBrush);
                break;

            case GatePaletteKind.Input:
                DrawInput(context, P, pen);
                break;

            case GatePaletteKind.Output:
                DrawOutput(context, P, pen);
                break;

            case GatePaletteKind.ConstantZero:
                DrawConstant(context, "0", P, pen, textBrush, scale);
                break;

            case GatePaletteKind.ConstantOne:
                DrawConstant(context, "1", P, pen, textBrush, scale);
                break;
        }
    }

    private static void DrawNot(DrawingContext context, Func<double, double, Point> p, double scale, Pen pen)
    {
        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(p(15, 0), false);
            stream.LineTo(p(15, 50));
            stream.LineTo(p(70, 25));
            stream.EndFigure(true);
        }

        context.DrawGeometry(null, pen, geometry);
        context.DrawEllipse(null, pen, p(75, 25), 5 * scale, 5 * scale);
        context.DrawLine(pen, p(0, 25), p(15, 25));
        context.DrawLine(pen, p(80, 25), p(100, 25));
    }

    private static void DrawAnd(
        DrawingContext context,
        bool inverted,
        int inputCount,
        Func<double, double, Point> p,
        double scale,
        Pen pen)
    {
        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(p(55, 0), false);
            stream.LineTo(p(15, 0));
            stream.LineTo(p(15, 50));
            stream.LineTo(p(55, 50));
            stream.CubicBezierTo(p(88.333, 50), p(88.333, 0), p(55, 0));
            stream.EndFigure(false);
        }

        context.DrawGeometry(null, pen, geometry);

        if (inverted)
        {
            context.DrawEllipse(null, pen, p(83.5, 25), 4.5 * scale, 4.5 * scale);
            context.DrawLine(pen, p(88, 25), p(100, 25));
        }
        else
        {
            context.DrawLine(pen, p(80, 25), p(100, 25));
        }

        DrawInputLines(context, inputCount, p, pen, 15);
    }

    private static void DrawOr(
        DrawingContext context,
        bool inverted,
        bool xor,
        int inputCount,
        Func<double, double, Point> p,
        double scale,
        Pen pen)
    {
        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(p(15, 0), false);
            stream.CubicBezierTo(p(35, 0), p(60, 5), p(80, 25));
            stream.CubicBezierTo(p(60, 45), p(35, 50), p(15, 50));
            stream.CubicBezierTo(p(25, 38), p(25, 12), p(15, 0));
            stream.EndFigure(false);
        }

        context.DrawGeometry(null, pen, geometry);

        if (xor)
        {
            var xorGeometry = new StreamGeometry();
            using (var stream = xorGeometry.Open())
            {
                stream.BeginFigure(p(8, 0), false);
                stream.CubicBezierTo(p(18, 12), p(18, 38), p(8, 50));
                stream.EndFigure(false);
            }

            context.DrawGeometry(null, pen, xorGeometry);
        }

        if (inverted)
        {
            context.DrawEllipse(null, pen, p(84.5, 25), 4.5 * scale, 4.5 * scale);
            context.DrawLine(pen, p(89, 25), p(100, 25));
        }
        else
        {
            context.DrawLine(pen, p(80, 25), p(100, 25));
        }

        DrawInputLines(context, inputCount, p, pen, xor ? 18 : 15);
    }

    private static void DrawMux(DrawingContext context, Func<double, double, Point> p, double scale, Pen pen, IBrush textBrush)
    {
        context.DrawRectangle(null, pen, new Rect(p(15, 0), p(65, 50)));
        context.DrawLine(pen, p(0, 10), p(15, 10));
        context.DrawLine(pen, p(0, 25), p(15, 25));
        context.DrawLine(pen, p(0, 40), p(15, 40));
        context.DrawLine(pen, p(65, 25), p(100, 25));
        DrawText(context, "D0", p(18, 3), textBrush, 9 * scale);
        DrawText(context, "D1", p(18, 18), textBrush, 9 * scale);
        DrawText(context, "S", p(18, 33), textBrush, 9 * scale);
        DrawText(context, "OUT", p(40, 18), textBrush, 8 * scale);
    }

    private static void DrawInput(DrawingContext context, Func<double, double, Point> p, Pen pen)
    {
        context.DrawLine(pen, p(40, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));
    }

    private static void DrawOutput(DrawingContext context, Func<double, double, Point> p, Pen pen)
    {
        context.DrawLine(pen, p(0, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));
    }

    private static void DrawConstant(DrawingContext context, string value, Func<double, double, Point> p, Pen pen, IBrush textBrush, double scale)
    {
        context.DrawLine(pen, p(55, 25), p(40, 25));
        context.DrawLine(pen, p(40, 20), p(40, 30));
        DrawText(context, value, p(22, 14), textBrush, 15 * scale);
    }

    private static void DrawInputLines(DrawingContext context, int inputCount, Func<double, double, Point> p, Pen pen, double targetX)
    {
        var offset = inputCount switch
        {
            2 => 30,
            3 => 15,
            4 => 10,
            _ => 30
        };

        for (var index = 0; index < inputCount; index++)
        {
            var y = 10 + index * offset;
            context.DrawLine(pen, p(0, y), p(targetX, y));
        }
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
        var formattedText = CreateText(text, brush, fontSize);
        var point = new Point(
            bounds.X + (bounds.Width - formattedText.Width) / 2,
            bounds.Y + (bounds.Height - formattedText.Height) / 2);

        context.DrawText(formattedText, point);
    }

    private static void DrawText(DrawingContext context, string text, Point point, IBrush brush, double fontSize)
    {
        context.DrawText(CreateText(text, brush, fontSize), point);
    }

    private static FormattedText CreateText(string text, IBrush brush, double fontSize)
    {
        var effectiveFontSize = Math.Max(1, fontSize);

        return new FormattedText(
            text,
            CultureInfo.CurrentCulture,
            FlowDirection.LeftToRight,
            Typeface.Default,
            effectiveFontSize,
            brush);
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
