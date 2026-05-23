using System.Globalization;
using Avalonia;
using Avalonia.Media;
using LogicFriday1.Models;

namespace LogicFriday1.Controls;

internal static class GateSymbolRenderer
{
    public static void Draw(
        DrawingContext context,
        GatePaletteKind kind,
        int inputCount,
        string label,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush,
        double scale = 1)
    {
        switch (kind)
        {
            case GatePaletteKind.Not:
                DrawNot(context, p, pen, scale);
                break;

            case GatePaletteKind.Nand:
                DrawAnd(context, true, inputCount, p, pen);
                break;

            case GatePaletteKind.And:
                DrawAnd(context, false, inputCount, p, pen);
                break;

            case GatePaletteKind.Nor:
                DrawOr(context, true, false, inputCount, p, pen, scale);
                break;

            case GatePaletteKind.Or:
                DrawOr(context, false, false, inputCount, p, pen, scale);
                break;

            case GatePaletteKind.Xor:
                DrawOr(context, false, true, inputCount, p, pen, scale);
                break;

            case GatePaletteKind.Mux:
                DrawMux(context, p, pen, textBrush, scale);
                break;

            case GatePaletteKind.ConstantZero:
                DrawConstant(context, "0", p, pen, textBrush, scale);
                break;

            case GatePaletteKind.ConstantOne:
                DrawConstant(context, "1", p, pen, textBrush, scale);
                break;

            case GatePaletteKind.Input:
                DrawInput(context, label, p, pen, textBrush, scale);
                break;

            case GatePaletteKind.Output:
                DrawOutput(context, label, p, pen, textBrush, scale);
                break;
        }
    }

    public static FormattedText CreateText(string text, IBrush brush, double fontSize)
    {
        return new FormattedText(
            text,
            CultureInfo.CurrentCulture,
            FlowDirection.LeftToRight,
            Typeface.Default,
            Math.Max(1, fontSize),
            brush);
    }

    public static void DrawText(DrawingContext context, string text, Point point, IBrush brush, double fontSize)
    {
        context.DrawText(CreateText(text, brush, fontSize), point);
    }

    private static void DrawNot(DrawingContext context, Func<double, double, Point> p, Pen pen, double scale)
    {
        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(p(15, 5), false);
            stream.LineTo(p(15, 45));
            stream.LineTo(p(55, 25));
            stream.EndFigure(true);
        }

        context.DrawGeometry(null, pen, geometry);
        context.DrawEllipse(null, pen, p(59.5, 25), 4.5 * scale, 4.5 * scale);
        context.DrawLine(pen, p(0, 25), p(15, 25));
        context.DrawLine(pen, p(64, 25), p(100, 25));
    }

    private static void DrawAnd(
        DrawingContext context,
        bool inverted,
        int inputCount,
        Func<double, double, Point> p,
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
            context.DrawEllipse(null, pen, p(83.5, 25), 4.5, 4.5);
            context.DrawLine(pen, p(88, 25), p(100, 25));
        }
        else
        {
            context.DrawLine(pen, p(80, 25), p(100, 25));
        }

        DrawInputLines(context, inputCount, p, pen, 15, []);
    }

    private static void DrawOr(
        DrawingContext context,
        bool inverted,
        bool xor,
        int inputCount,
        Func<double, double, Point> p,
        Pen pen,
        double scale)
    {
        var inputOffset = xor ? -7 : 0;
        var inputAdjustments = GetOrInputAdjustments(inputCount, xor);

        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(p(35, 0), false);
            stream.LineTo(p(15, 0));
            stream.CubicBezierTo(p(25, 12), p(25, 38), p(15, 50));
            stream.LineTo(p(35, 50));
            stream.CubicBezierTo(p(50, 50), p(75, 37), p(80, 25));
            stream.CubicBezierTo(p(75, 13), p(55, 0), p(35, 0));
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

        DrawInputLines(context, inputCount, p, pen, 15 + inputOffset, inputAdjustments);
    }

    private static void DrawMux(
        DrawingContext context,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush,
        double scale)
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

    private static void DrawConstant(
        DrawingContext context,
        string value,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush,
        double scale)
    {
        context.DrawLine(pen, p(55, 25), p(40, 25));
        context.DrawLine(pen, p(40, 20), p(40, 30));
        DrawText(context, value, p(22, 14), textBrush, 15 * scale);
    }

    private static void DrawInput(
        DrawingContext context,
        string label,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush,
        double scale)
    {
        context.DrawLine(pen, p(40, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));

        if (label.Length == 0)
        {
            return;
        }

        var text = CreateText(label, textBrush, 12 * scale);
        context.DrawText(text, p(16 - text.Width / scale, 13));
    }

    private static void DrawOutput(
        DrawingContext context,
        string label,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush,
        double scale)
    {
        context.DrawLine(pen, p(0, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));

        if (label.Length > 0)
        {
            DrawText(context, label, p(26, 13), textBrush, 12 * scale);
        }
    }

    private static void DrawInputLines(
        DrawingContext context,
        int inputCount,
        Func<double, double, Point> p,
        Pen pen,
        double targetX,
        IReadOnlyList<double> targetAdjustments)
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
            var adjustment = index < targetAdjustments.Count ? targetAdjustments[index] : 0;
            context.DrawLine(pen, p(0, y), p(targetX + adjustment, y));
        }
    }

    private static IReadOnlyList<double> GetOrInputAdjustments(int inputCount, bool xor)
    {
        if (xor)
        {
            return inputCount switch
            {
                2 => [4, 4],
                3 => [0, 0, 4],
                4 => [7, 4, 0, 4],
                _ => []
            };
        }

        return inputCount switch
        {
            2 => [5.261, 5.261],
            3 => [5.261, 7.5, 5.261],
            4 => [5.261, 7.267, 7.267, 5.261],
            _ => []
        };
    }
}
