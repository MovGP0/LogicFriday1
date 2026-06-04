using System.Globalization;
using System.Security;
using System.Text;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class GateDiagramSvgExportService
{
    private const double ItemWidth = 100;

    private const double ItemHeight = 66;

    private const double Padding = 24;

    public static bool CanExport(LogicFunction? logicFunction)
    {
        return logicFunction is GateDiagramFunction { Items.Count: > 0 };
    }

    public static string Export(GateDiagramFunction logicFunction)
    {
        if (logicFunction.Items.Count == 0)
        {
            throw new InvalidOperationException("Gate diagram export requires at least one diagram item.");
        }

        var bounds = GetBounds(logicFunction).Inflate(Padding);
        var builder = new StringBuilder();
        builder
            .Append("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"")
            .Append(Format(bounds.Left))
            .Append(' ')
            .Append(Format(bounds.Top))
            .Append(' ')
            .Append(Format(bounds.Width))
            .Append(' ')
            .Append(Format(bounds.Height))
            .Append("\" width=\"")
            .Append(Format(bounds.Width))
            .Append("\" height=\"")
            .Append(Format(bounds.Height))
            .AppendLine("\">")
            .AppendLine("  <rect x=\"0\" y=\"0\" width=\"100%\" height=\"100%\" fill=\"white\"/>")
            .AppendLine("  <g fill=\"none\" stroke=\"black\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">");

        foreach (var wire in logicFunction.Wires)
        {
            if (!TryGetWirePoints(logicFunction.Items, wire, out var points))
            {
                continue;
            }

            builder
                .Append("    <polyline points=\"")
                .Append(string.Join(" ", points.Select(static point => $"{Format(point.X)},{Format(point.Y)}")))
                .AppendLine("\"/>");
        }

        foreach (var item in logicFunction.Items)
        {
            AppendItem(builder, item);
        }

        builder
            .AppendLine("  </g>")
            .AppendLine("</svg>");

        return builder.ToString();
    }

    private static void AppendItem(StringBuilder builder, GateDiagramItem item)
    {
        switch (item.Kind)
        {
            case GatePaletteKind.Input:
            case GatePaletteKind.Output:
                AppendTerminal(builder, item);
                break;

            case GatePaletteKind.ConstantZero:
            case GatePaletteKind.ConstantOne:
                AppendConstant(builder, item);
                break;

            default:
                AppendGate(builder, item);
                break;
        }

        if (!string.IsNullOrWhiteSpace(item.ComponentLabel))
        {
            AppendText(builder, item.ComponentLabel, item.X + 16, item.Y - 6, 10);
        }
    }

    private static void AppendGate(StringBuilder builder, GateDiagramItem item)
    {
        var label = item.Kind.ToString().ToUpperInvariant();
        builder
            .Append("    <rect x=\"")
            .Append(Format(item.X + 15))
            .Append("\" y=\"")
            .Append(Format(item.Y + 6))
            .Append("\" width=\"68\" height=\"44\" rx=\"4\"/>")
            .AppendLine();
        AppendPins(builder, item);
        AppendText(builder, label, item.X + 28, item.Y + 32, 12);
    }

    private static void AppendTerminal(StringBuilder builder, GateDiagramItem item)
    {
        var label = item.Kind == GatePaletteKind.Output
            ? "OUTPUT"
            : item.Label;
        var y = item.Y + 25;
        builder
            .Append("    <line x1=\"")
            .Append(Format(item.X))
            .Append("\" y1=\"")
            .Append(Format(y))
            .Append("\" x2=\"")
            .Append(Format(item.X + 40))
            .Append("\" y2=\"")
            .Append(Format(y))
            .AppendLine("\"/>");
        AppendText(builder, label, item.X + 46, item.Y + 30, 12);
    }

    private static void AppendConstant(StringBuilder builder, GateDiagramItem item)
    {
        builder
            .Append("    <line x1=\"")
            .Append(Format(item.X + 40))
            .Append("\" y1=\"")
            .Append(Format(item.Y + 25))
            .Append("\" x2=\"")
            .Append(Format(item.X + 55))
            .Append("\" y2=\"")
            .Append(Format(item.Y + 25))
            .AppendLine("\"/>");
        AppendText(builder, item.Kind == GatePaletteKind.ConstantOne ? "1" : "0", item.X + 24, item.Y + 30, 14);
    }

    private static void AppendPins(StringBuilder builder, GateDiagramItem item)
    {
        foreach (var input in GetInputConnections(item))
        {
            builder
                .Append("    <line x1=\"")
                .Append(Format(item.X))
                .Append("\" y1=\"")
                .Append(Format(input.Y))
                .Append("\" x2=\"")
                .Append(Format(input.X + 15))
                .Append("\" y2=\"")
                .Append(Format(input.Y))
                .AppendLine("\"/>");
        }

        if (TryGetOutputConnection(item, out var output))
        {
            builder
                .Append("    <line x1=\"")
                .Append(Format(item.X + 83))
                .Append("\" y1=\"")
                .Append(Format(output.Y))
                .Append("\" x2=\"")
                .Append(Format(output.X))
                .Append("\" y2=\"")
                .Append(Format(output.Y))
                .AppendLine("\"/>");
        }
    }

    private static void AppendText(
        StringBuilder builder,
        string text,
        double x,
        double y,
        double fontSize)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return;
        }

        builder
            .Append("    <text x=\"")
            .Append(Format(x))
            .Append("\" y=\"")
            .Append(Format(y))
            .Append("\" fill=\"black\" stroke=\"none\" font-family=\"Arial, sans-serif\" font-size=\"")
            .Append(Format(fontSize))
            .Append("\">")
            .Append(SecurityElement.Escape(text))
            .AppendLine("</text>");
    }

    private static Rect GetBounds(GateDiagramFunction logicFunction)
    {
        Rect? bounds = null;
        foreach (var item in logicFunction.Items)
        {
            AddBounds(new Rect(item.X, item.Y, ItemWidth, ItemHeight));
        }

        foreach (var wire in logicFunction.Wires)
        {
            if (!TryGetWirePoints(logicFunction.Items, wire, out var points))
            {
                continue;
            }

            foreach (var point in points)
            {
                AddBounds(new Rect(point.X, point.Y, 1, 1));
            }
        }

        return bounds ?? new Rect(0, 0, ItemWidth, ItemHeight);

        void AddBounds(Rect rect)
        {
            bounds = bounds is { } existing ? existing.Union(rect) : rect;
        }
    }

    private static bool TryGetWirePoints(
        IReadOnlyList<GateDiagramItem> items,
        GateDiagramWire wire,
        out IReadOnlyList<Point> points)
    {
        points = [];
        if (!TryResolveConnection(items, wire.Start, out var start) ||
            !TryResolveConnection(items, wire.End, out var end))
        {
            return false;
        }

        points =
        [
            start,
            .. wire.RoutePoints.Select(static point => new Point(point.X, point.Y)),
            end
        ];
        return true;
    }

    private static bool TryResolveConnection(
        IReadOnlyList<GateDiagramItem> items,
        GateDiagramConnectionReference reference,
        out Point point)
    {
        var item = items.FirstOrDefault(candidate => candidate.Id == reference.ItemId);
        if (item is null)
        {
            point = default;
            return false;
        }

        if (reference.Kind == GateDiagramConnectionKind.Output)
        {
            return TryGetOutputConnection(item, out point);
        }

        point = GetInputConnections(item)
            .ElementAtOrDefault(reference.PinIndex);
        return point != default;
    }

    private static IReadOnlyList<Point> GetInputConnections(GateDiagramItem item)
    {
        return item.Kind switch
        {
            GatePaletteKind.Not or GatePaletteKind.Output => [new Point(item.X, item.Y + 25)],
            GatePaletteKind.Mux =>
            [
                new Point(item.X, item.Y + 10),
                new Point(item.X, item.Y + 25),
                new Point(item.X, item.Y + 40)
            ],
            GatePaletteKind.Nand or
            GatePaletteKind.And or
            GatePaletteKind.Nor or
            GatePaletteKind.Or or
            GatePaletteKind.Xor => Enumerable
                .Range(0, Math.Max(1, item.InputCount))
                .Select(index => new Point(item.X, item.Y + 10 + index * GetInputOffset(item.InputCount)))
                .ToArray(),
            _ => []
        };
    }

    private static bool TryGetOutputConnection(GateDiagramItem item, out Point point)
    {
        point = item.Kind switch
        {
            GatePaletteKind.Input => new Point(item.X + 40, item.Y + 25),
            GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne => new Point(item.X + 55, item.Y + 25),
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.And or
            GatePaletteKind.Nor or
            GatePaletteKind.Or or
            GatePaletteKind.Xor or
            GatePaletteKind.Mux => new Point(item.X + 100, item.Y + 25),
            _ => default
        };

        return point != default;
    }

    private static double GetInputOffset(int inputCount)
    {
        return inputCount switch
        {
            3 => 15,
            4 => 10,
            _ => 30
        };
    }

    private static string Format(double value)
    {
        return value.ToString("0.###", CultureInfo.InvariantCulture);
    }

    private readonly record struct Point(double X, double Y);

    private readonly record struct Rect(double X, double Y, double Width, double Height)
    {
        public double Left => X;

        public double Top => Y;

        public double Right => X + Width;

        public double Bottom => Y + Height;

        public Rect Inflate(double padding)
        {
            return new Rect(X - padding, Y - padding, Width + padding * 2, Height + padding * 2);
        }

        public Rect Union(Rect other)
        {
            var left = Math.Min(Left, other.Left);
            var top = Math.Min(Top, other.Top);
            var right = Math.Max(Right, other.Right);
            var bottom = Math.Max(Bottom, other.Bottom);
            return new Rect(left, top, right - left, bottom - top);
        }
    }
}
