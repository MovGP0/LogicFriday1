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

        GateSymbolRenderer.Draw(context, item.Kind, item.InputCount, item.Label, P, pen, textBrush);

        if (item.ComponentLabel.Length > 0)
        {
            DrawCenteredText(context, item.ComponentLabel, P, 55, textBrush, 11);
        }
    }

    private static void DrawCenteredText(
        DrawingContext context,
        string text,
        Func<double, double, Point> p,
        double y,
        IBrush brush,
        double fontSize)
    {
        var formattedText = GateSymbolRenderer.CreateText(text, brush, fontSize);
        context.DrawText(formattedText, p((100 - formattedText.Width) / 2, y));
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
            label.Trim(),
            GetNextComponentLabel(item)));

        InvalidateVisual();
    }

    private string GetNextComponentLabel(GatePaletteItem item)
    {
        if (!HasComponentLabel(item.Kind))
        {
            return string.Empty;
        }

        var nextNumber = (Items ?? [])
            .Count(static diagramItem => diagramItem.ComponentLabel.Length > 0) + 1;

        return $"[{nextNumber}]";
    }

    private static bool HasComponentLabel(GatePaletteKind kind)
    {
        return kind is
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.Nor or
            GatePaletteKind.Mux or
            GatePaletteKind.And or
            GatePaletteKind.Or or
            GatePaletteKind.Xor;
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
