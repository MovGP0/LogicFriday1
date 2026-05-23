using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Threading;
using LogicFriday1.Models;

namespace LogicFriday1.Controls;

public sealed class GateDiagramSurface : Control
{
    public static readonly StyledProperty<GatePaletteItem?> SelectedPaletteItemProperty =
        AvaloniaProperty.Register<GateDiagramSurface, GatePaletteItem?>(nameof(SelectedPaletteItem));

    public static readonly StyledProperty<IList<GateDiagramItem>?> ItemsProperty =
        AvaloniaProperty.Register<GateDiagramSurface, IList<GateDiagramItem>?>(nameof(Items));

    public static readonly StyledProperty<IList<GateDiagramWire>?> WiresProperty =
        AvaloniaProperty.Register<GateDiagramSurface, IList<GateDiagramWire>?>(nameof(Wires));

    private const double ConnectionHitRadius = 6;
    private const double WireGeometryTolerance = 0.001;

    private GateDiagramConnectionPoint? _pendingWireStart;
    private Point? _pendingWirePreviewEnd;
    private Point? _invalidWirePoint;
    private DispatcherTimer? _invalidWireTimer;
    private int _nextItemId = 1;

    public GateDiagramSurface()
    {
        Focusable = true;
    }

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

    public IList<GateDiagramWire>? Wires
    {
        get => GetValue(WiresProperty);
        set => SetValue(WiresProperty, value);
    }

    public event EventHandler<GateDiagramVariableNameRequestedEventArgs>? VariableNameRequested;

    public event EventHandler? PaletteSelectionCleared;

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        if (SelectedPaletteItem is not { } item ||
            !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            return;
        }

        if (item.Kind == GatePaletteKind.Wire)
        {
            BeginWire(e);
            return;
        }

        if (Items is null || !IsPlaceable(item))
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

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);

        if (_pendingWireStart is null)
        {
            return;
        }

        var position = e.GetPosition(this);
        _pendingWirePreviewEnd = TryHitConnection(position, out var hitConnection)
            ? new Point(hitConnection.X, hitConnection.Y)
            : position;

        InvalidateVisual();
        e.Handled = true;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);

        if (e.Key != Key.Escape || _pendingWireStart is null)
        {
            return;
        }

        CancelPendingWire();
        e.Handled = true;
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
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

        DrawWires(context, new Pen(Brushes.Black, 1.5));

        foreach (var item in Items ?? [])
        {
            DrawItem(context, item);
        }

        DrawInvalidWireTarget(context);
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

    private void DrawWires(DrawingContext context, Pen pen)
    {
        foreach (var wire in Wires ?? [])
        {
            if (TryResolveConnection(wire.Start, out var wireStart) &&
                TryResolveConnection(wire.End, out var wireEnd))
            {
                DrawWireRoute(
                    context,
                    pen,
                    CreateOrthogonalRoute(
                        new Point(wireStart.X, wireStart.Y),
                        new Point(wireEnd.X, wireEnd.Y)));
            }
        }

        if (_pendingWireStart is { } start && _pendingWirePreviewEnd is { } end)
        {
            var previewPen = new Pen(Brushes.Black, 1.2, DashStyle.Dash);
            var points = CreateOrthogonalRoute(new Point(start.X, start.Y), end);
            DrawWireRoute(context, previewPen, points);
        }
    }

    private static void DrawWireRoute(
        DrawingContext context,
        Pen pen,
        IReadOnlyList<Point> points)
    {
        for (var index = 1; index < points.Count; index++)
        {
            context.DrawLine(
                pen,
                points[index - 1],
                points[index]);
        }
    }

    private void DrawInvalidWireTarget(DrawingContext context)
    {
        if (_invalidWirePoint is not { } point)
        {
            return;
        }

        var pen = new Pen(Brushes.Firebrick, 1.8);
        context.DrawEllipse(null, pen, point, 7, 7);
        context.DrawLine(pen, new Point(point.X - 5, point.Y + 5), new Point(point.X + 5, point.Y - 5));
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

    private void BeginWire(PointerPressedEventArgs e)
    {
        if (Wires is null)
        {
            return;
        }

        Focus();

        var position = e.GetPosition(this);
        if (_pendingWireStart is null)
        {
            if (!TryHitConnection(position, out var start))
            {
                ShowInvalidWireTarget(position);
                e.Handled = true;
                return;
            }

            _pendingWireStart = start;
            _pendingWirePreviewEnd = new Point(start.X, start.Y);
            _invalidWirePoint = null;
            InvalidateVisual();
            e.Handled = true;
            return;
        }

        if (!TryHitConnection(position, out var end) ||
            !CanConnect(_pendingWireStart, end))
        {
            ShowInvalidWireTarget(position);
            e.Handled = true;
            return;
        }

        Wires.Add(new GateDiagramWire(_pendingWireStart.Reference, end.Reference));
        ClearPaletteSelection();
        InvalidateVisual();
        e.Handled = true;
    }

    private void ClearPaletteSelection()
    {
        _pendingWireStart = null;
        _pendingWirePreviewEnd = null;
        _invalidWirePoint = null;
        SelectedPaletteItem = null;
        PaletteSelectionCleared?.Invoke(this, EventArgs.Empty);
    }

    private void CancelPendingWire()
    {
        ClearPaletteSelection();
        InvalidateVisual();
    }

    private void AddItem(GatePaletteItem item, double x, double y, string label)
    {
        Items?.Add(new GateDiagramItem(
            item.Kind,
            item.InputCount,
            x,
            y,
            label.Trim(),
            GetNextComponentLabel(item),
            _nextItemId++));

        ClearPaletteSelection();
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

    private bool TryHitConnection(Point position, out GateDiagramConnectionPoint connection)
    {
        connection = default!;
        var bestDistanceSquared = ConnectionHitRadius * ConnectionHitRadius;
        var found = false;

        foreach (var candidate in EnumerateConnectionPoints())
        {
            var distanceSquared =
                Math.Pow(candidate.X - position.X, 2) +
                Math.Pow(candidate.Y - position.Y, 2);

            if (distanceSquared > bestDistanceSquared)
            {
                continue;
            }

            connection = candidate;
            bestDistanceSquared = distanceSquared;
            found = true;
        }

        return found;
    }

    private IEnumerable<GateDiagramConnectionPoint> EnumerateConnectionPoints()
    {
        if (Items is null)
        {
            yield break;
        }

        foreach (var item in Items)
        {
            foreach (var input in GetInputConnections(item))
            {
                yield return input;
            }

            if (TryGetOutputConnection(item, out var output))
            {
                yield return output;
            }
        }
    }

    private static IEnumerable<GateDiagramConnectionPoint> GetInputConnections(
        GateDiagramItem item)
    {
        switch (item.Kind)
        {
            case GatePaletteKind.Not:
                yield return CreateInput(item, 0, 25);
                break;

            case GatePaletteKind.Nand:
            case GatePaletteKind.And:
            case GatePaletteKind.Nor:
            case GatePaletteKind.Or:
            case GatePaletteKind.Xor:
                var offset = item.InputCount switch
                {
                    2 => 30,
                    3 => 15,
                    4 => 10,
                    _ => 30
                };

                for (var inputIndex = 0; inputIndex < item.InputCount; inputIndex++)
                {
                    yield return CreateInput(item, inputIndex, 10 + inputIndex * offset);
                }

                break;

            case GatePaletteKind.Mux:
                yield return CreateInput(item, 0, 10);
                yield return CreateInput(item, 1, 25);
                yield return CreateInput(item, 2, 40);
                break;

            case GatePaletteKind.Output:
                yield return CreateInput(item, 0, 25);
                break;
        }
    }

    private static GateDiagramConnectionPoint CreateInput(
        GateDiagramItem item,
        int pinIndex,
        double y)
    {
        return new GateDiagramConnectionPoint(
            new GateDiagramConnectionReference(
                item.Id,
                GateDiagramConnectionKind.Input,
                pinIndex),
            item.X,
            item.Y + y);
    }

    private static bool TryGetOutputConnection(
        GateDiagramItem item,
        out GateDiagramConnectionPoint connection)
    {
        Point? position = item.Kind switch
        {
            GatePaletteKind.Not or
            GatePaletteKind.Nand or
            GatePaletteKind.And or
            GatePaletteKind.Nor or
            GatePaletteKind.Or or
            GatePaletteKind.Xor or
            GatePaletteKind.Mux => new Point(item.X + 100, item.Y + 25),
            GatePaletteKind.Input => new Point(item.X + 40, item.Y + 25),
            GatePaletteKind.ConstantZero or
            GatePaletteKind.ConstantOne => new Point(item.X + 55, item.Y + 25),
            _ => null
        };

        if (position is not { } outputPosition)
        {
            connection = default!;
            return false;
        }

        connection = new GateDiagramConnectionPoint(
            new GateDiagramConnectionReference(
                item.Id,
                GateDiagramConnectionKind.Output,
                0),
            outputPosition.X,
            outputPosition.Y);

        return true;
    }

    private bool CanConnect(GateDiagramConnectionPoint start, GateDiagramConnectionPoint end)
    {
        if (IsSameConnection(start, end))
        {
            return false;
        }

        if (start.Reference.Kind == GateDiagramConnectionKind.Output &&
            end.Reference.Kind == GateDiagramConnectionKind.Output)
        {
            return false;
        }

        return Wires is null ||
            !Wires.Any(wire => ConnectsSamePair(wire, start, end));
    }

    private static bool ConnectsSamePair(
        GateDiagramWire wire,
        GateDiagramConnectionPoint start,
        GateDiagramConnectionPoint end)
    {
        return (IsSameConnection(wire.Start, start.Reference) && IsSameConnection(wire.End, end.Reference)) ||
            (IsSameConnection(wire.Start, end.Reference) && IsSameConnection(wire.End, start.Reference));
    }

    private static bool IsSameConnection(
        GateDiagramConnectionPoint left,
        GateDiagramConnectionPoint right)
    {
        return IsSameConnection(left.Reference, right.Reference);
    }

    private static bool IsSameConnection(
        GateDiagramConnectionReference left,
        GateDiagramConnectionReference right)
    {
        return left.ItemId == right.ItemId &&
            left.Kind == right.Kind &&
            left.PinIndex == right.PinIndex;
    }

    private bool TryResolveConnection(
        GateDiagramConnectionReference reference,
        out GateDiagramConnectionPoint connection)
    {
        var match = EnumerateConnectionPoints()
            .FirstOrDefault(candidate => IsSameConnection(candidate.Reference, reference));

        if (match is null)
        {
            connection = default!;
            return false;
        }

        connection = match;
        return true;
    }

    private static IReadOnlyList<Point> CreateOrthogonalRoute(Point start, Point end)
    {
        var points = new List<Point>
        {
            start
        };

        if (Math.Abs(start.X - end.X) > WireGeometryTolerance &&
            Math.Abs(start.Y - end.Y) > WireGeometryTolerance)
        {
            points.Add(new Point(end.X, start.Y));
        }

        points.Add(end);
        return points;
    }

    private void ShowInvalidWireTarget(Point point)
    {
        _invalidWirePoint = point;
        _invalidWireTimer ??= new DispatcherTimer
        {
            Interval = TimeSpan.FromMilliseconds(650)
        };

        _invalidWireTimer.Stop();
        _invalidWireTimer.Tick -= InvalidWireTimerOnTick;
        _invalidWireTimer.Tick += InvalidWireTimerOnTick;
        _invalidWireTimer.Start();
        InvalidateVisual();
    }

    private void InvalidWireTimerOnTick(object? sender, EventArgs e)
    {
        if (_invalidWireTimer is not null)
        {
            _invalidWireTimer.Stop();
            _invalidWireTimer.Tick -= InvalidWireTimerOnTick;
        }

        _invalidWirePoint = null;
        InvalidateVisual();
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
