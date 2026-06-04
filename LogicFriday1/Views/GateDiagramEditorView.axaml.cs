using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using LogicFriday1.Controls;
using LogicFriday1.Models;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class GateDiagramEditorView : UserControl
{
    private const string ActiveGatePaletteButtonClass = "active";

    private Button? _activeGatePaletteButton;

    public GateDiagramEditorView()
    {
        InitializeComponent();
        GateDiagramSurface.VariableNameRequested += GateDiagramSurface_OnVariableNameRequested;
        GateDiagramSurface.PaletteSelectionCleared += GateDiagramSurface_OnPaletteSelectionCleared;
    }

    public event EventHandler? SubmitRequested;

    public event EventHandler? CancelRequested;

    public event EventHandler? HelpRequested;

    public event EventHandler<GateDiagramVariableNameRequestedEventArgs>? VariableNameRequested;

    public void ZoomIn()
    {
        GateDiagramSurface.ZoomIn();
    }

    public void ZoomOut()
    {
        GateDiagramSurface.ZoomOut();
    }

    public void ZoomAll()
    {
        var contentBounds = GateDiagramSurface.ZoomAll(GateDiagramScrollViewer.Bounds.Size);
        GateDiagramScrollViewer.Offset = new Vector(
            Math.Max(0, contentBounds.Left * GateDiagramSurface.Zoom),
            Math.Max(0, contentBounds.Top * GateDiagramSurface.Zoom));
    }

    public int AutoRedraw()
    {
        return GateDiagramSurface.AutoRedraw();
    }

    public void DeleteSelected()
    {
        GateDiagramSurface.DeleteSelected();
    }

    public void CancelInteraction()
    {
        GateDiagramSurface.CancelInteraction();
    }

    public void ClearActivePaletteButton()
    {
        if (_activeGatePaletteButton is null)
        {
            return;
        }

        _activeGatePaletteButton.Classes.Remove(ActiveGatePaletteButtonClass);
        _activeGatePaletteButton = null;
    }

    private void GatePaletteButton_OnClick(object? sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: GatePaletteItem item } button ||
            DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        if (item.Kind == GatePaletteKind.Help)
        {
            HelpRequested?.Invoke(this, EventArgs.Empty);
            return;
        }

        if (item.Kind == GatePaletteKind.Cancel)
        {
            CancelRequested?.Invoke(this, EventArgs.Empty);
            return;
        }

        if (item.Kind == GatePaletteKind.Submit)
        {
            SubmitRequested?.Invoke(this, EventArgs.Empty);
            return;
        }

        SetActiveGatePaletteButton(button);
        viewModel.SelectGatePaletteItem(item);
    }

    private void GateDiagramSurface_OnPaletteSelectionCleared(object? sender, EventArgs e)
    {
        ClearActivePaletteButton();

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ClearGatePaletteSelection();
        }
    }

    private void SetActiveGatePaletteButton(Button button)
    {
        if (ReferenceEquals(_activeGatePaletteButton, button))
        {
            return;
        }

        ClearActivePaletteButton();
        button.Classes.Add(ActiveGatePaletteButtonClass);
        _activeGatePaletteButton = button;
    }

    private void GateDiagramSurface_OnVariableNameRequested(
        object? sender,
        GateDiagramVariableNameRequestedEventArgs e)
    {
        VariableNameRequested?.Invoke(this, e);
    }
}
