using System.Globalization;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using LogicFriday1.Models;

namespace LogicFriday1.Controls;

public sealed class GateDiagramSurface : Control
{
    public static readonly StyledProperty<GatePaletteItem?> SelectedPaletteItemProperty =
        AvaloniaProperty.Register<GateDiagramSurface, GatePaletteItem?>(nameof(SelectedPaletteItem));

    public static readonly StyledProperty<IList<GateDiagramItem>?> ItemsProperty =
        AvaloniaProperty.Register<GateDiagramSurface, IList<GateDiagramItem>?>(nameof(Items));

    public GatePaletteItem? SelectedPaletteItem
    {
        get => GetValue(SelectedPaletteItemProperty);
        set => SetValue(SelectedPaletteItemProperty, value);
    }

    public IList<GateDiagramItem>? Items
    {
        get => GetValue(ItemsProperty);
        set => SetValue(ItemsProperty, value);
    }

    public event EventHandler<GateDiagramVariableNameRequestedEventArgs>? VariableNameRequested;

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        if (SelectedPaletteItem is not { } item ||
            Items is null ||
            !IsPlaceable(item) ||
            !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            return;
        }

        var position = e.GetPosition(this);
        var x = Snap(position.X - 50);
        var y = Snap(position.Y - 25);
        if (item.Kind is GatePaletteKind.Input or GatePaletteKind.Output)
        {
            VariableNameRequested?.Invoke(
                this,
                new GateDiagramVariableNameRequestedEventArgs(
                    item,
                    x,
                    y,
                    name => AddItem(item, x, y, name)));

            e.Handled = true;
            return;
        }

        AddItem(item, x, y, item.Label);
        e.Handled = true;
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);

        var bounds = new Rect(0, 0, Bounds.Width, Bounds.Height);
        var backgroundBrush = FindBrush("LogicFriday.Brush.SurfaceContainerLowest", Brushes.White);
        var gridPen = new Pen(FindBrush("LogicFriday.Brush.OutlineVariant", Brushes.LightGray), 1);
        var borderPen = new Pen(FindBrush("LogicFriday.Brush.Outline", Brushes.Gray), 1);

        context.FillRectangle(backgroundBrush, bounds);
        DrawGrid(context, bounds, gridPen);
        context.DrawRectangle(null, borderPen, bounds.Deflate(0.5));

        foreach (var item in Items ?? [])
        {
            DrawItem(context, item);
        }
    }

    private static void DrawGrid(DrawingContext context, Rect bounds, Pen pen)
    {
        const double spacing = 20;

        for (var x = spacing; x < bounds.Width; x += spacing)
        {
            context.DrawLine(pen, new Point(x, 0), new Point(x, bounds.Height));
        }

        for (var y = spacing; y < bounds.Height; y += spacing)
        {
            context.DrawLine(pen, new Point(0, y), new Point(bounds.Width, y));
        }
    }

    private static void DrawItem(DrawingContext context, GateDiagramItem item)
    {
        var pen = new Pen(Brushes.Black, 1.5);
        var textBrush = Brushes.Black;
        var origin = new Point(item.X, item.Y);

        Point P(double x, double y)
        {
            return new Point(origin.X + x, origin.Y + y);
        }

        switch (item.Kind)
        {
            case GatePaletteKind.Not:
                DrawNot(context, P, pen);
                break;

            case GatePaletteKind.Nand:
                DrawAnd(context, true, item.InputCount, P, pen);
                break;

            case GatePaletteKind.And:
                DrawAnd(context, false, item.InputCount, P, pen);
                break;

            case GatePaletteKind.Nor:
                DrawOr(context, true, false, item.InputCount, P, pen);
                break;

            case GatePaletteKind.Or:
                DrawOr(context, false, false, item.InputCount, P, pen);
                break;

            case GatePaletteKind.Xor:
                DrawOr(context, false, true, item.InputCount, P, pen);
                break;

            case GatePaletteKind.Mux:
                DrawMux(context, P, pen, textBrush);
                break;

            case GatePaletteKind.ConstantZero:
                DrawConstant(context, "0", P, pen, textBrush);
                break;

            case GatePaletteKind.ConstantOne:
                DrawConstant(context, "1", P, pen, textBrush);
                break;

            case GatePaletteKind.Input:
                DrawInput(context, item.Label, P, pen, textBrush);
                break;

            case GatePaletteKind.Output:
                DrawOutput(context, item.Label, P, pen, textBrush);
                break;
        }
    }

    private static void DrawNot(DrawingContext context, Func<double, double, Point> p, Pen pen)
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
        context.DrawEllipse(null, pen, p(59.5, 25), 4.5, 4.5);
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
            stream.CubicBezierTo(p(80, 50), p(80, 0), p(55, 0));
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
        Pen pen)
    {
        var inputOffset = xor ? -7 : 0;
        var inputAdjustments = GetOrInputAdjustments(inputCount);

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
            context.DrawEllipse(null, pen, p(84.5, 25), 4.5, 4.5);
            context.DrawLine(pen, p(89, 25), p(100, 25));
        }
        else
        {
            context.DrawLine(pen, p(80, 25), p(100, 25));
        }

        DrawInputLines(context, inputCount, p, pen, 15 + inputOffset, inputAdjustments);
    }

    private static void DrawMux(DrawingContext context, Func<double, double, Point> p, Pen pen, IBrush textBrush)
    {
        context.DrawRectangle(null, pen, new Rect(p(15, 0), p(65, 50)));
        context.DrawLine(pen, p(0, 10), p(15, 10));
        context.DrawLine(pen, p(0, 25), p(15, 25));
        context.DrawLine(pen, p(0, 40), p(15, 40));
        context.DrawLine(pen, p(65, 25), p(100, 25));
        DrawText(context, "D0", p(18, 3), textBrush, 9);
        DrawText(context, "D1", p(18, 18), textBrush, 9);
        DrawText(context, "S", p(18, 33), textBrush, 9);
        DrawText(context, "OUT", p(40, 18), textBrush, 8);
    }

    private static void DrawConstant(
        DrawingContext context,
        string value,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush)
    {
        context.DrawLine(pen, p(55, 25), p(40, 25));
        context.DrawLine(pen, p(40, 20), p(40, 30));
        DrawText(context, value, p(22, 14), textBrush, 15);
    }

    private static void DrawInput(
        DrawingContext context,
        string label,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush)
    {
        context.DrawLine(pen, p(40, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));

        var text = CreateText(label, textBrush, 12);
        context.DrawText(text, p(16 - text.Width, 13));
    }

    private static void DrawOutput(
        DrawingContext context,
        string label,
        Func<double, double, Point> p,
        Pen pen,
        IBrush textBrush)
    {
        context.DrawLine(pen, p(0, 25), p(20, 25));
        context.DrawLine(pen, p(20, 15), p(20, 35));
        DrawText(context, label, p(26, 13), textBrush, 12);
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

    private static void DrawText(DrawingContext context, string text, Point point, IBrush brush, double fontSize)
    {
        context.DrawText(CreateText(text, brush, fontSize), point);
    }

    private static FormattedText CreateText(string text, IBrush brush, double fontSize)
    {
        return new FormattedText(
            text,
            CultureInfo.CurrentCulture,
            FlowDirection.LeftToRight,
            Typeface.Default,
            fontSize,
            brush);
    }

    private static IReadOnlyList<double> GetOrInputAdjustments(int inputCount)
    {
        return inputCount switch
        {
            2 => [4, 4],
            3 => [0, 0, 4],
            4 => [7, 4, 0, 4],
            _ => []
        };
    }

    private static bool IsPlaceable(GatePaletteItem item)
    {
        return item.Kind is
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.Nor or
            GatePaletteKind.Mux or
            GatePaletteKind.And or
            GatePaletteKind.Or or
            GatePaletteKind.Xor or
            GatePaletteKind.ConstantZero or
            GatePaletteKind.ConstantOne or
            GatePaletteKind.Input or
            GatePaletteKind.Output;
    }

    private void AddItem(GatePaletteItem item, double x, double y, string label)
    {
        Items?.Add(new GateDiagramItem(
            item.Kind,
            item.InputCount,
            x,
            y,
            label.Trim()));

        InvalidateVisual();
    }

    private static double Snap(double value)
    {
        const double spacing = 20;
        return Math.Round(value / spacing) * spacing;
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

public sealed class GateDiagramVariableNameRequestedEventArgs(
    GatePaletteItem item,
    double x,
    double y,
    Action<string> addItem) : EventArgs
{
    public GatePaletteItem Item { get; } = item;

    public double X { get; } = x;

    public double Y { get; } = y;

    public void AddItem(string variableName)
    {
        addItem(variableName);
    }
}
