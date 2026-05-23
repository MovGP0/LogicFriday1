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

    public static readonly StyledProperty<double> ZoomProperty =
        AvaloniaProperty.Register<GateDiagramSurface, double>(nameof(Zoom), 1d);

    private const double ConnectionHitRadius = 6;
    private const double WireGeometryTolerance = 0.001;
    private const double MinimumZoom = 0.25;
    private const double MaximumZoom = 4;
    private const double ZoomStep = 1.2;
    private const double LogicalCanvasWidth = 2400;
    private const double LogicalCanvasHeight = 1600;

    private GateDiagramConnectionPoint? _pendingWireStart;
    private Point? _pendingWirePreviewEnd;
    private Point? _invalidWirePoint;
    private readonly HashSet<int> _selectedItemIds = [];
    private readonly HashSet<int> _selectedWireIndices = [];
    private int? _selectedWireSegmentIndex;
    private bool _isDraggingSelection;
    private bool _isDraggingSelectionRectangle;
    private Point _lastSelectionDragPoint;
    private Point _selectionRectangleStart;
    private Point _selectionRectangleEnd;
    private DispatcherTimer? _invalidWireTimer;
    private int _nextItemId = 1;

    public GateDiagramSurface()
    {
        Focusable = true;
    }

    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property == ZoomProperty)
        {
            InvalidateMeasure();
            InvalidateVisual();
        }
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

    public double Zoom
    {
        get => GetValue(ZoomProperty);
        set => SetValue(ZoomProperty, ClampZoom(value));
    }

    public event EventHandler<GateDiagramVariableNameRequestedEventArgs>? VariableNameRequested;

    public event EventHandler? PaletteSelectionCleared;

    public void CancelInteraction()
    {
        CancelPendingWireState();
        ClearSelection();
        SelectedPaletteItem = null;
        InvalidateVisual();
    }

    public void ZoomIn()
    {
        Zoom *= ZoomStep;
    }

    public void ZoomOut()
    {
        Zoom /= ZoomStep;
    }

    public Rect ZoomAll(Size viewportSize)
    {
        var contentBounds = GetContentBounds().Inflate(80);
        var availableWidth = Math.Max(1, viewportSize.Width);
        var availableHeight = Math.Max(1, viewportSize.Height);

        Zoom = Math.Min(
            availableWidth / Math.Max(1, contentBounds.Width),
            availableHeight / Math.Max(1, contentBounds.Height));

        return contentBounds;
    }

    protected override Size MeasureOverride(Size availableSize)
    {
        var contentBounds = GetContentBounds();
        return new Size(
            Math.Max(LogicalCanvasWidth, contentBounds.Right + 120) * Zoom,
            Math.Max(LogicalCanvasHeight, contentBounds.Bottom + 120) * Zoom);
    }

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

        if (item.Kind == GatePaletteKind.Select)
        {
            BeginSelect(e);
            return;
        }

        if (Items is null || !IsPlaceable(item))
        {
            return;
        }

        var position = GetLogicalPosition(e);
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
            MoveSelection(e);
            return;
        }

        var position = GetLogicalPosition(e);
        _pendingWirePreviewEnd = TryHitConnection(position, out var hitConnection)
            ? new Point(hitConnection.X, hitConnection.Y)
            : position;

        InvalidateVisual();
        e.Handled = true;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);

        if (e.Key == Key.Delete)
        {
            DeleteSelection();
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Escape || _pendingWireStart is null)
        {
            return;
        }

        CancelPendingWire();
        e.Handled = true;
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);

        if (!e.KeyModifiers.HasFlag(KeyModifiers.Control))
        {
            return;
        }

        if (e.Delta.Y > 0)
        {
            ZoomIn();
        }
        else if (e.Delta.Y < 0)
        {
            ZoomOut();
        }

        e.Handled = true;
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);

        if (!_isDraggingSelection)
        {
            return;
        }

        if (_isDraggingSelectionRectangle)
        {
            SelectItemsInRectangle(GetSelectionRectangle());
            _isDraggingSelectionRectangle = false;
        }

        _isDraggingSelection = false;
        e.Pointer.Capture(null);
        InvalidateVisual();
        e.Handled = true;
    }

    public override void Render(DrawingContext context)
    {
        base.Render(context);

        var bounds = new Rect(0, 0, Bounds.Width / Zoom, Bounds.Height / Zoom);
        var backgroundBrush = FindBrush("LogicFriday.Brush.SurfaceContainerLowest", Brushes.White);
        var gridPen = new Pen(FindBrush("LogicFriday.Brush.OutlineVariant", Brushes.LightGray), 1);
        var borderPen = new Pen(FindBrush("LogicFriday.Brush.Outline", Brushes.Gray), 1);

        using var zoomScope = context.PushTransform(Matrix.CreateScale(Zoom, Zoom));
        context.FillRectangle(backgroundBrush, bounds);
        DrawGrid(context, bounds, gridPen);
        context.DrawRectangle(null, borderPen, bounds.Deflate(0.5));

        DrawWires(context, new Pen(Brushes.Black, 1.5));

        foreach (var item in Items ?? [])
        {
            DrawItem(context, item);
        }

        DrawInvalidWireTarget(context);
        DrawSelectionRectangle(context);
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

    private void DrawItem(DrawingContext context, GateDiagramItem item)
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

        if (_selectedItemIds.Contains(item.Id))
        {
            var selectionPen = new Pen(Brushes.Firebrick, 1.4, DashStyle.Dash);
            context.DrawRectangle(null, selectionPen, GetItemBounds(item).Inflate(4));
        }
    }

    private void DrawWires(DrawingContext context, Pen pen)
    {
        var wireIndex = 0;
        foreach (var wire in Wires ?? [])
        {
            if (TryResolveConnection(wire.Start, out var wireStart) &&
                TryResolveConnection(wire.End, out var wireEnd))
            {
                var points = GetWireRoute(wire, wireStart, wireEnd);
                var wirePen = _selectedWireIndices.Contains(wireIndex)
                    ? new Pen(Brushes.Firebrick, 2.4)
                    : pen;

                DrawWireRoute(context, wirePen, points);

                if (_selectedWireIndices.Contains(wireIndex) &&
                    _selectedWireSegmentIndex is { } selectedSegmentIndex)
                {
                    DrawSelectedWireSegment(context, points, selectedSegmentIndex);
                }
            }

            wireIndex++;
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

    private static void DrawSelectedWireSegment(
        DrawingContext context,
        IReadOnlyList<Point> points,
        int segmentIndex)
    {
        if (segmentIndex < 0 || segmentIndex + 1 >= points.Count)
        {
            return;
        }

        context.DrawLine(
            new Pen(Brushes.Firebrick, 4.2),
            points[segmentIndex],
            points[segmentIndex + 1]);
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

    private void DrawSelectionRectangle(DrawingContext context)
    {
        if (!_isDraggingSelectionRectangle)
        {
            return;
        }

        var pen = new Pen(Brushes.Firebrick, 1.2, DashStyle.Dash);
        context.DrawRectangle(null, pen, GetSelectionRectangle());
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
        ClearSelection();

        var position = GetLogicalPosition(e);
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

    private void BeginSelect(PointerPressedEventArgs e)
    {
        if (Items is null)
        {
            return;
        }

        Focus();
        CancelPendingWireState();

        var position = GetLogicalPosition(e);
        if (TryHitItem(position, out var item))
        {
            if (!_selectedItemIds.Contains(item.Id))
            {
                ClearSelection();
                _selectedItemIds.Add(item.Id);
            }

            _selectedWireSegmentIndex = null;
            BeginSelectionDrag(e, position);
            return;
        }

        if (TryHitWire(position, out var wireIndex, out var segmentIndex))
        {
            if (!_selectedWireIndices.Contains(wireIndex))
            {
                ClearSelection();
                _selectedWireIndices.Add(wireIndex);
            }

            _selectedWireSegmentIndex = segmentIndex;
            BeginSelectionDrag(e, position);
            return;
        }

        ClearSelection();
        _isDraggingSelectionRectangle = true;
        _selectionRectangleStart = position;
        _selectionRectangleEnd = position;
        BeginSelectionDrag(e, position);
        InvalidateVisual();
    }

    private void BeginSelectionDrag(PointerPressedEventArgs e, Point position)
    {
        _isDraggingSelection = true;
        _lastSelectionDragPoint = Snap(position);
        e.Pointer.Capture(this);
        InvalidateVisual();
        e.Handled = true;
    }

    private void MoveSelection(PointerEventArgs e)
    {
        if (!_isDraggingSelection)
        {
            return;
        }

        if (_isDraggingSelectionRectangle)
        {
            _selectionRectangleEnd = GetLogicalPosition(e);
            InvalidateVisual();
            e.Handled = true;
            return;
        }

        var position = Snap(GetLogicalPosition(e));
        var delta = position - _lastSelectionDragPoint;
        if (Math.Abs(delta.X) < WireGeometryTolerance &&
            Math.Abs(delta.Y) < WireGeometryTolerance)
        {
            e.Handled = true;
            return;
        }

        _lastSelectionDragPoint = position;

        if (_selectedItemIds.Count > 0)
        {
            MoveSelectedItems(delta);
        }
        else if (_selectedWireIndices.Count == 1 &&
            _selectedWireIndices.FirstOrDefault() is var selectedWireIndex &&
            _selectedWireSegmentIndex is { } selectedWireSegmentIndex)
        {
            MoveSelectedWireSegment(selectedWireIndex, selectedWireSegmentIndex, delta);
        }

        InvalidateVisual();
        e.Handled = true;
    }

    private void ClearPaletteSelection()
    {
        CancelPendingWireState();
        SelectedPaletteItem = null;
        PaletteSelectionCleared?.Invoke(this, EventArgs.Empty);
    }

    private void CancelPendingWire()
    {
        ClearPaletteSelection();
        InvalidateVisual();
    }

    private void CancelPendingWireState()
    {
        _pendingWireStart = null;
        _pendingWirePreviewEnd = null;
        _invalidWirePoint = null;
    }

    private void ClearSelection()
    {
        _selectedItemIds.Clear();
        _selectedWireIndices.Clear();
        _selectedWireSegmentIndex = null;
        _isDraggingSelection = false;
        _isDraggingSelectionRectangle = false;
    }

    private Point GetLogicalPosition(PointerEventArgs e)
    {
        return ToLogical(e.GetPosition(this));
    }

    private Point ToLogical(Point point)
    {
        return new Point(point.X / Zoom, point.Y / Zoom);
    }

    private static double ClampZoom(double zoom)
    {
        if (double.IsNaN(zoom) || double.IsInfinity(zoom))
        {
            return 1;
        }

        return Math.Clamp(zoom, MinimumZoom, MaximumZoom);
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

    private static Point Snap(Point point)
    {
        return new Point(Snap(point.X), Snap(point.Y));
    }

    private bool TryHitItem(Point position, out GateDiagramItem item)
    {
        item = default!;
        if (Items is null)
        {
            return false;
        }

        for (var index = Items.Count - 1; index >= 0; index--)
        {
            var candidate = Items[index];
            if (!GetItemBounds(candidate).Contains(position))
            {
                continue;
            }

            item = candidate;
            return true;
        }

        return false;
    }

    private static Rect GetItemBounds(GateDiagramItem item)
    {
        return item.Kind switch
        {
            GatePaletteKind.Input => new Rect(item.X, item.Y, 50, 50),
            GatePaletteKind.Output => new Rect(item.X, item.Y, 60, 50),
            GatePaletteKind.ConstantZero or GatePaletteKind.ConstantOne => new Rect(item.X, item.Y, 60, 50),
            _ => new Rect(item.X, item.Y, 100, 66)
        };
    }

    private bool TryHitWire(Point position, out int wireIndex, out int segmentIndex)
    {
        wireIndex = -1;
        segmentIndex = -1;
        if (Wires is null)
        {
            return false;
        }

        var bestDistance = 7d;
        for (var index = 0; index < Wires.Count; index++)
        {
            var wire = Wires[index];
            if (!TryResolveConnection(wire.Start, out var start) ||
                !TryResolveConnection(wire.End, out var end))
            {
                continue;
            }

            var points = GetWireRoute(wire, start, end);
            for (var routeIndex = 0; routeIndex + 1 < points.Count; routeIndex++)
            {
                var distance = DistanceToOrthogonalSegment(position, points[routeIndex], points[routeIndex + 1]);
                if (distance > bestDistance)
                {
                    continue;
                }

                bestDistance = distance;
                wireIndex = index;
                segmentIndex = routeIndex;
            }
        }

        return wireIndex >= 0;
    }

    private static double DistanceToOrthogonalSegment(Point point, Point start, Point end)
    {
        if (Math.Abs(start.Y - end.Y) < WireGeometryTolerance)
        {
            var left = Math.Min(start.X, end.X);
            var right = Math.Max(start.X, end.X);
            if (point.X < left || point.X > right)
            {
                return double.PositiveInfinity;
            }

            return Math.Abs(point.Y - start.Y);
        }

        if (Math.Abs(start.X - end.X) < WireGeometryTolerance)
        {
            var top = Math.Min(start.Y, end.Y);
            var bottom = Math.Max(start.Y, end.Y);
            if (point.Y < top || point.Y > bottom)
            {
                return double.PositiveInfinity;
            }

            return Math.Abs(point.X - start.X);
        }

        return double.PositiveInfinity;
    }

    private Rect GetSelectionRectangle()
    {
        return new Rect(_selectionRectangleStart, _selectionRectangleEnd).Normalize();
    }

    private Rect GetContentBounds()
    {
        Rect? bounds = null;

        foreach (var item in Items ?? [])
        {
            AddBounds(GetItemBounds(item));
        }

        foreach (var wire in Wires ?? [])
        {
            if (!TryResolveConnection(wire.Start, out var start) ||
                !TryResolveConnection(wire.End, out var end))
            {
                continue;
            }

            foreach (var point in GetWireRoute(wire, start, end))
            {
                AddBounds(new Rect(point, new Size(1, 1)).Inflate(10));
            }
        }

        return bounds ?? new Rect(0, 0, LogicalCanvasWidth, LogicalCanvasHeight);

        void AddBounds(Rect rect)
        {
            bounds = bounds is { } existing ? existing.Union(rect) : rect;
        }
    }

    private void SelectItemsInRectangle(Rect selectionRectangle)
    {
        ClearSelection();

        if (Items is not null)
        {
            foreach (var item in Items)
            {
                if (selectionRectangle.Contains(GetItemBounds(item)))
                {
                    _selectedItemIds.Add(item.Id);
                }
            }
        }

        if (Wires is not null)
        {
            for (var index = 0; index < Wires.Count; index++)
            {
                if (WireIntersectsRectangle(Wires[index], selectionRectangle))
                {
                    _selectedWireIndices.Add(index);
                }
            }
        }
    }

    private void MoveSelectedItems(Vector delta)
    {
        if (Items is null)
        {
            return;
        }

        for (var index = 0; index < Items.Count; index++)
        {
            if (!_selectedItemIds.Contains(Items[index].Id))
            {
                continue;
            }

            var item = Items[index];
            Items[index] = item with
            {
                X = item.X + delta.X,
                Y = item.Y + delta.Y
            };
        }
    }

    private void DeleteSelection()
    {
        if (_selectedItemIds.Count == 0 && _selectedWireIndices.Count == 0)
        {
            return;
        }

        DeleteSelectedWires();
        DeleteSelectedItems();
        ClearSelection();
        InvalidateVisual();
    }

    private void DeleteSelectedWires()
    {
        if (Wires is null)
        {
            return;
        }

        for (var index = Wires.Count - 1; index >= 0; index--)
        {
            var wire = Wires[index];
            if (_selectedWireIndices.Contains(index) ||
                _selectedItemIds.Contains(wire.Start.ItemId) ||
                _selectedItemIds.Contains(wire.End.ItemId))
            {
                Wires.RemoveAt(index);
            }
        }
    }

    private void DeleteSelectedItems()
    {
        if (Items is null)
        {
            return;
        }

        for (var index = Items.Count - 1; index >= 0; index--)
        {
            if (_selectedItemIds.Contains(Items[index].Id))
            {
                Items.RemoveAt(index);
            }
        }
    }

    private bool WireIntersectsRectangle(GateDiagramWire wire, Rect rectangle)
    {
        if (!TryResolveConnection(wire.Start, out var start) ||
            !TryResolveConnection(wire.End, out var end))
        {
            return false;
        }

        var points = GetWireRoute(wire, start, end);
        for (var index = 1; index < points.Count; index++)
        {
            if (SegmentIntersectsRectangle(points[index - 1], points[index], rectangle))
            {
                return true;
            }
        }

        return false;
    }

    private static bool SegmentIntersectsRectangle(Point start, Point end, Rect rectangle)
    {
        if (rectangle.Contains(start) || rectangle.Contains(end))
        {
            return true;
        }

        if (Math.Abs(start.Y - end.Y) < WireGeometryTolerance)
        {
            var segmentLeft = Math.Min(start.X, end.X);
            var segmentRight = Math.Max(start.X, end.X);
            return start.Y >= rectangle.Top &&
                start.Y <= rectangle.Bottom &&
                segmentRight >= rectangle.Left &&
                segmentLeft <= rectangle.Right;
        }

        if (Math.Abs(start.X - end.X) < WireGeometryTolerance)
        {
            var segmentTop = Math.Min(start.Y, end.Y);
            var segmentBottom = Math.Max(start.Y, end.Y);
            return start.X >= rectangle.Left &&
                start.X <= rectangle.Right &&
                segmentBottom >= rectangle.Top &&
                segmentTop <= rectangle.Bottom;
        }

        return false;
    }

    private void MoveSelectedWireSegment(int wireIndex, int segmentIndex, Vector delta)
    {
        if (Wires is null ||
            wireIndex < 0 ||
            wireIndex >= Wires.Count)
        {
            return;
        }

        var wire = Wires[wireIndex];
        if (!TryResolveConnection(wire.Start, out var start) ||
            !TryResolveConnection(wire.End, out var end))
        {
            return;
        }

        var route = GetWireRoute(wire, start, end).ToList();
        if (segmentIndex < 0 || segmentIndex + 1 >= route.Count)
        {
            return;
        }

        var segmentStart = route[segmentIndex];
        var segmentEnd = route[segmentIndex + 1];
        if (Math.Abs(segmentStart.Y - segmentEnd.Y) < WireGeometryTolerance)
        {
            MoveHorizontalSegment(route, segmentIndex, delta.Y);
        }
        else if (Math.Abs(segmentStart.X - segmentEnd.X) < WireGeometryTolerance)
        {
            MoveVerticalSegment(route, segmentIndex, delta.X);
        }

        route = NormalizeRoute(OrthogonalizeRoute(route));
        Wires[wireIndex] = wire with
        {
            RoutePoints = route
                .Skip(1)
                .Take(Math.Max(0, route.Count - 2))
                .Select(static point => new GateDiagramWirePoint(point.X, point.Y))
                .ToArray()
        };
    }

    private static void MoveHorizontalSegment(List<Point> route, int segmentIndex, double deltaY)
    {
        if (Math.Abs(deltaY) < WireGeometryTolerance)
        {
            return;
        }

        var newY = route[segmentIndex].Y + deltaY;
        if (segmentIndex == 0)
        {
            route.Insert(1, new Point(route[0].X, newY));
            segmentIndex++;
        }

        if (segmentIndex + 1 == route.Count - 1)
        {
            route.Insert(route.Count - 1, new Point(route[^1].X, newY));
        }

        route[segmentIndex] = new Point(route[segmentIndex].X, newY);
        route[segmentIndex + 1] = new Point(route[segmentIndex + 1].X, newY);
    }

    private static void MoveVerticalSegment(List<Point> route, int segmentIndex, double deltaX)
    {
        if (Math.Abs(deltaX) < WireGeometryTolerance)
        {
            return;
        }

        var newX = route[segmentIndex].X + deltaX;
        if (segmentIndex == 0)
        {
            route.Insert(1, new Point(newX, route[0].Y));
            segmentIndex++;
        }

        if (segmentIndex + 1 == route.Count - 1)
        {
            route.Insert(route.Count - 1, new Point(newX, route[^1].Y));
        }

        route[segmentIndex] = new Point(newX, route[segmentIndex].Y);
        route[segmentIndex + 1] = new Point(newX, route[segmentIndex + 1].Y);
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

    private static IReadOnlyList<Point> GetWireRoute(
        GateDiagramWire wire,
        GateDiagramConnectionPoint start,
        GateDiagramConnectionPoint end)
    {
        var startPoint = new Point(start.X, start.Y);
        var endPoint = new Point(end.X, end.Y);
        if (wire.RoutePoints.Count == 0)
        {
            return CreateOrthogonalRoute(startPoint, endPoint);
        }

        var route = new List<Point> { startPoint };
        route.AddRange(wire.RoutePoints.Select(static point => new Point(point.X, point.Y)));
        route.Add(endPoint);
        return NormalizeRoute(OrthogonalizeRoute(route));
    }

    private static List<Point> OrthogonalizeRoute(IReadOnlyList<Point> route)
    {
        var points = new List<Point>();
        foreach (var target in route)
        {
            if (points.Count == 0)
            {
                points.Add(target);
                continue;
            }

            var current = points[^1];
            if (AreSamePoint(current, target))
            {
                continue;
            }

            if (Math.Abs(current.X - target.X) > WireGeometryTolerance &&
                Math.Abs(current.Y - target.Y) > WireGeometryTolerance)
            {
                points.Add(new Point(target.X, current.Y));
            }

            points.Add(target);
        }

        return points;
    }

    private static List<Point> NormalizeRoute(IEnumerable<Point> route)
    {
        var points = new List<Point>();
        foreach (var point in route)
        {
            if (points.Count > 0 &&
                AreSamePoint(points[^1], point))
            {
                continue;
            }

            points.Add(point);
        }

        for (var index = 1; index + 1 < points.Count;)
        {
            var previous = points[index - 1];
            var current = points[index];
            var next = points[index + 1];
            if ((Math.Abs(previous.X - current.X) < WireGeometryTolerance &&
                    Math.Abs(current.X - next.X) < WireGeometryTolerance) ||
                (Math.Abs(previous.Y - current.Y) < WireGeometryTolerance &&
                    Math.Abs(current.Y - next.Y) < WireGeometryTolerance))
            {
                points.RemoveAt(index);
                continue;
            }

            index++;
        }

        return points;
    }

    private static bool AreSamePoint(Point left, Point right)
    {
        return Math.Abs(left.X - right.X) < WireGeometryTolerance &&
            Math.Abs(left.Y - right.Y) < WireGeometryTolerance;
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
