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

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        if (SelectedPaletteItem is not { Kind: GatePaletteKind.Nand, InputCount: 2 } item ||
            Items is null ||
            !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            return;
        }

        var position = e.GetPosition(this);
        Items.Add(new GateDiagramItem(
            item.Kind,
            item.InputCount,
            Snap(position.X - 50),
            Snap(position.Y - 25),
            item.Label));

        InvalidateVisual();
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
        if (item.Kind == GatePaletteKind.Nand && item.InputCount == 2)
        {
            DrawNand2(context, item);
        }
    }

    private static void DrawNand2(DrawingContext context, GateDiagramItem item)
    {
        var pen = new Pen(Brushes.Black, 1.5);
        var textBrush = Brushes.Black;
        var x = item.X;
        var y = item.Y;

        var geometry = new StreamGeometry();
        using (var stream = geometry.Open())
        {
            stream.BeginFigure(new Point(x + 55, y), false);
            stream.LineTo(new Point(x + 15, y));
            stream.LineTo(new Point(x + 15, y + 50));
            stream.LineTo(new Point(x + 55, y + 50));
            stream.ArcTo(
                new Point(x + 55, y),
                new Size(25, 25),
                0,
                false,
                SweepDirection.CounterClockwise);
            stream.EndFigure(false);
        }

        context.DrawGeometry(null, pen, geometry);
        context.DrawEllipse(null, pen, new Point(x + 83.5, y + 25), 4.5, 4.5);
        context.DrawLine(pen, new Point(x + 88, y + 25), new Point(x + 100, y + 25));
        context.DrawLine(pen, new Point(x, y + 10), new Point(x + 15, y + 10));
        context.DrawLine(pen, new Point(x, y + 40), new Point(x + 15, y + 40));
        context.DrawText(
            new FormattedText(
                item.Label,
                CultureInfo.CurrentCulture,
                FlowDirection.LeftToRight,
                Typeface.Default,
                11,
                textBrush),
            new Point(x + 22, y + 55));
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
