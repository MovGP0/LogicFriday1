using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Input.Platform;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using LogicFriday1.Controls;
using LogicFriday1.Models;
using LogicFriday1.Services;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MainWindow : Window
{
    private const string HelpContentsUrl = "https://github.com/MovGP0/LogicFriday1/wiki";
    private const string GateDiagramHelpUrl = "https://github.com/MovGP0/LogicFriday1/wiki/Entering-a-gate-diagram";
    private INotifyPropertyChanged? _propertyChangedDataContext;

    public MainWindow()
    {
        InitializeComponent();
        DataContextChanged += MainWindow_OnDataContextChanged;
    }

    private void MainWindow_OnDataContextChanged(object? sender, EventArgs e)
    {
        if (_propertyChangedDataContext is not null)
        {
            _propertyChangedDataContext.PropertyChanged -= DataContext_OnPropertyChanged;
        }

        _propertyChangedDataContext = DataContext as INotifyPropertyChanged;
        if (_propertyChangedDataContext is not null)
        {
            _propertyChangedDataContext.PropertyChanged += DataContext_OnPropertyChanged;
        }

        UpdateTruthTableMainMenuState();
    }

    private void DataContext_OnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(MainWindowViewModel.IsTruthTableVisible))
        {
            UpdateTruthTableMainMenuState();
        }

        if (e.PropertyName == nameof(MainWindowViewModel.IsEquationEditorVisible))
        {
            RefreshEquationEditorToolbarButtons();
        }
    }

    private async void HelpContents_OnClick(object? sender, RoutedEventArgs e)
    {
        await OpenUrlAsync(HelpContentsUrl, "Help contents could not be opened.");
    }

    private async void AboutLogicFriday_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new AboutDialog();
        await dialog.ShowDialog(this);
    }

    private void UnminimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowUnminimizedView();
        }
    }

    private void MinimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowMinimizedView();
        }
    }

    private async void Minimize_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsOperationMinimizeEnabled: true } viewModel ||
            viewModel.GetSelectedFunction() is not { } selectedFunction)
        {
            return;
        }

        var dialog = new MinimizeDialog
        {
            OutputCount = selectedFunction.OutputNames.Length
        };
        var result = await dialog.ShowDialog<bool?>(this);
        if (result != true)
        {
            return;
        }

        viewModel.MinimizeSelectedFunction(dialog.ViewModel.ToMinimizeOptions());
        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private async void MapToGates_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsOperationMapToGatesEnabled: true } viewModel)
        {
            return;
        }

        if (viewModel.GetSelectedFunction() is { MinimizedFunction: null } selectedFunction)
        {
            var minimizeDialog = new MinimizeDialog
            {
                OutputCount = selectedFunction.OutputNames.Length
            };
            var minimizeResult = await minimizeDialog.ShowDialog<bool?>(this);
            if (minimizeResult != true)
            {
                return;
            }

            if (!viewModel.EnsureSelectedFunctionMinimized(minimizeDialog.ViewModel.ToMinimizeOptions(), out var minimizeErrorMessage) &&
                !string.IsNullOrWhiteSpace(minimizeErrorMessage))
            {
                await ShowMessageAsync(minimizeErrorMessage, "Map to Gates");
                return;
            }
        }

        var dialog = new MapToGatesDialog();
        var result = await dialog.ShowDialog<bool?>(this);
        if (result != true)
        {
            return;
        }

        if (!viewModel.MapSelectedFunctionToGates(dialog.ViewModel, out var errorMessage) &&
            !string.IsNullOrWhiteSpace(errorMessage))
        {
            await ShowMessageAsync(errorMessage, "Map to Gates");
        }
    }

    private async void GenerateLookupFunction_OnClick(object? sender, RoutedEventArgs e)
    {
        if (sender is not MenuItem { Tag: string languageTag } ||
            !Enum.TryParse<LookupFunctionLanguage>(languageTag, out var language) ||
            DataContext is not MainWindowViewModel { IsOperationGenerateLookupFunctionEnabled: true } viewModel ||
            viewModel.GetSelectedFunction() is not { } selectedFunction)
        {
            return;
        }

        var generated = LookupFunctionGenerator.Generate(selectedFunction, language);
        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = $"Generate {LookupFunctionGenerator.GetDisplayName(language)} Lookup Function",
            SuggestedFileName = generated.FileName,
            DefaultExtension = generated.FileExtension.TrimStart('.'),
            FileTypeChoices =
            [
                new FilePickerFileType(generated.FileTypeName)
                {
                    Patterns = [ $"*{generated.FileExtension}" ]
                },
                new FilePickerFileType("All Files")
                {
                    Patterns = [ "*" ]
                }
            ]
        });

        if (file is null)
        {
            return;
        }

        try
        {
            await using var output = await file.OpenWriteAsync();
            if (output.CanSeek)
            {
                output.SetLength(0);
            }

            await using var writer = new StreamWriter(output);
            await writer.WriteAsync(generated.SourceCode);
            viewModel.StatusText = $"{LookupFunctionGenerator.GetDisplayName(language)} lookup function generated";
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"The lookup function file could not be written.\n{ex.Message}", "Generate Lookup Function");
        }
    }

    private void CloneFunction_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsOperationCloneFunctionEnabled: true } viewModel &&
            viewModel.CloneSelectedFunction() &&
            viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private async void CompareFunctions_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsOperationCompareFunctionsEnabled: true } viewModel &&
            viewModel.CompareSelectedFunctions())
        {
            await ShowMessageAsync(viewModel.StatusText, "Compare Functions");
        }
    }

    private async void OrFunctions_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsOperationOrFunctionsEnabled: true } viewModel)
        {
            return;
        }

        if (!viewModel.OrSelectedFunctions(out var errorMessage))
        {
            await ShowMessageAsync(errorMessage ?? viewModel.StatusText, "OR Functions");
            return;
        }

        ConfigureSelectedFunctionTruthTableColumns(viewModel);
    }

    private async void AndFunctions_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsOperationAndFunctionsEnabled: true } viewModel)
        {
            return;
        }

        if (!viewModel.AndSelectedFunctions(out var errorMessage))
        {
            await ShowMessageAsync(errorMessage ?? viewModel.StatusText, "AND Functions");
            return;
        }

        ConfigureSelectedFunctionTruthTableColumns(viewModel);
    }

    private async void XorFunctions_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsOperationXorFunctionsEnabled: true } viewModel)
        {
            return;
        }

        if (!viewModel.XorSelectedFunctions(out var errorMessage))
        {
            await ShowMessageAsync(errorMessage ?? viewModel.StatusText, "XOR Functions");
            return;
        }

        ConfigureSelectedFunctionTruthTableColumns(viewModel);
    }

    private void OperationCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelOperation();
        }
    }

    private void ConfigureSelectedFunctionTruthTableColumns(MainWindowViewModel viewModel)
    {
        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void GateZoomIn_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramEditor.ZoomIn();
        }
    }

    private void GateZoomOut_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramEditor.ZoomOut();
        }
    }

    private void GateZoomAll_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGateDiagramVisible: true })
        {
            return;
        }

        GateDiagramEditor.ZoomAll();
    }

    private void GateAutoRedraw_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true } viewModel)
        {
            var reroutedWireCount = GateDiagramEditor.AutoRedraw();
            viewModel.StatusText = reroutedWireCount == 0
                ? "Gate diagram redrawn"
                : $"Gate diagram redrawn: {reroutedWireCount} wire routes reset";
        }
    }

    private async void NewTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        await StartNewTruthTableAsync();
    }

    private async Task StartNewTruthTableAsync()
    {
        var dialog = new TruthTableSetupDialog();
        var result = await dialog.ShowDialog<bool?>(this);
        if (result != true || DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.StartNewTruthTable(dialog.ViewModel.InputNames, dialog.ViewModel.OutputNames);
        TruthTableEditor.ConfigureColumns(dialog.ViewModel.InputNames, dialog.ViewModel.OutputNames);
        UpdateTruthTableMainMenuState();
    }

    private void ModifyTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel ||
            !viewModel.StartModifyTruthTable() ||
            viewModel.GetSelectedFunction() is not { } logicFunction)
        {
            return;
        }

        TruthTableEditor.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        UpdateTruthTableMainMenuState();
    }

    private void ShowTrueAndDontCareTruthTableRows_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowTrueAndDontCareTruthTableRows();
        }
    }

    private void ShowAllTruthTableRows_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowAllTruthTableRows();
        }
    }

    private void NewLogicEquation_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsFileNewEnabled: true } viewModel)
        {
            viewModel.StartNewLogicEquation();
            LogicEquationEditor.FocusEditor();
            RefreshEquationEditorToolbarButtons();
        }
    }

    private void ModifyLogicEquation_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel &&
            viewModel.StartModifyLogicEquation())
        {
            LogicEquationEditor.FocusEditor();
            RefreshEquationEditorToolbarButtons();
        }
    }

    private void EquationSumOfProducts_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.ShowSumOfProductsEquation();
        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void EquationProductOfSums_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.ShowProductOfSumsEquation();
        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void EquationFactor_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.FactorSelectedEquation();
        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void NewGateDiagram_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsFileNewEnabled: true } viewModel)
        {
            viewModel.StartNewGateDiagram();
        }
    }

    private void ModifyGateDiagram_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGatesModifyGateDiagramEnabled: true } viewModel)
        {
            return;
        }

        GateDiagramEditor.ClearActivePaletteButton();
        GateDiagramEditor.CancelInteraction();
        viewModel.StartModifySelectedGateDiagram();
    }

    private async void GateCopyToClipboard_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGatesCopyToClipboardEnabled: true } viewModel)
        {
            return;
        }

        var svg = viewModel.CreateGateDiagramClipboardSvg();
        if (svg is null)
        {
            return;
        }

        var clipboard = TopLevel.GetTopLevel(this)?.Clipboard;
        if (clipboard is null)
        {
            viewModel.StatusText = "Clipboard is not available";
            await ShowMessageAsync("Clipboard is not available.", "Copy Gate Diagram");
            return;
        }

        try
        {
            await clipboard.SetTextAsync(svg);
        }
        catch (Exception ex)
        {
            viewModel.StatusText = $"Copy failed: {ex.Message}";
            await ShowMessageAsync($"The gate diagram could not be copied.\n{ex.Message}", "Copy Gate Diagram");
        }
    }

    private async void GateIcPackageInfo_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGatesIcPackageInfoEnabled: true } viewModel)
        {
            return;
        }

        var packageInfo = viewModel.GetSelectedGatePackageInfo();
        if (packageInfo is null)
        {
            await ShowMessageAsync(viewModel.StatusText, "IC Package Info");
            return;
        }

        await ShowTextDialogAsync(packageInfo, "IC Package Info");
    }

    private async void GateTraceLogic_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        if (!viewModel.ToggleGateTraceLogic())
        {
            await ShowMessageAsync(viewModel.StatusText, "Trace Gate Logic");
            return;
        }

        if (viewModel.IsGatesTraceLogicChecked && viewModel.GateTraceOutputText.Length > 0)
        {
            viewModel.StatusText = $"Gate logic trace enabled: {viewModel.GateTraceOutputText}";
        }
    }

    private async void ToolbarNew_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsFileNewEnabled: true })
        {
            return;
        }

        var choice = await ShowNewFunctionChooserAsync();
        switch (choice)
        {
            case "TruthTable":
                await StartNewTruthTableAsync();
                break;
            case "LogicEquation":
                NewLogicEquation_OnClick(sender, e);
                break;
            case "GateDiagram":
                NewGateDiagram_OnClick(sender, e);
                break;
        }
    }

    private async Task<string?> ShowNewFunctionChooserAsync()
    {
        var dialog = new Window
        {
            Title = "New Function",
            Width = 320,
            Height = 190,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            CanResize = false
        };

        var truthTableButton = new Button
        {
            Content = "Truth Table",
            HorizontalAlignment = HorizontalAlignment.Stretch
        };
        var logicEquationButton = new Button
        {
            Content = "Logic Equation",
            HorizontalAlignment = HorizontalAlignment.Stretch
        };
        var gateDiagramButton = new Button
        {
            Content = "Gate Diagram",
            HorizontalAlignment = HorizontalAlignment.Stretch
        };
        var cancelButton = new Button
        {
            Content = "Cancel",
            MinWidth = 80,
            HorizontalAlignment = HorizontalAlignment.Right,
            IsCancel = true
        };

        truthTableButton.Click += (_, _) => dialog.Close("TruthTable");
        logicEquationButton.Click += (_, _) => dialog.Close("LogicEquation");
        gateDiagramButton.Click += (_, _) => dialog.Close("GateDiagram");
        cancelButton.Click += (_, _) => dialog.Close(null);

        dialog.Content = new Grid
        {
            RowDefinitions = new RowDefinitions("Auto,Auto,Auto,Auto"),
            Margin = new Avalonia.Thickness(16),
            RowSpacing = 8,
            Children =
            {
                truthTableButton,
                logicEquationButton,
                gateDiagramButton,
                cancelButton
            }
        };

        Grid.SetRow(logicEquationButton, 1);
        Grid.SetRow(gateDiagramButton, 2);
        Grid.SetRow(cancelButton, 3);

        return await dialog.ShowDialog<string?>(this);
    }

    private async void ImportTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Import Truth Table",
            AllowMultiple = false,
            FileTypeFilter =
            [
                new FilePickerFileType("Truth Table Files")
                {
                    Patterns = [ "*.csv", "*.txt", "*.*" ]
                }
            ]
        });

        var file = files.FirstOrDefault();
        if (file is null)
        {
            return;
        }

        try
        {
            await using var stream = await file.OpenReadAsync();
            using var reader = new StreamReader(stream);
            var import = TruthTableImporter.Import(await reader.ReadToEndAsync());

            viewModel.StartImportedTruthTable(import.InputNames, import.OutputNames, import.OutputValues);
            TruthTableEditor.ConfigureColumns(import.InputNames, import.OutputNames);
            UpdateTruthTableMainMenuState();
        }
        catch (TruthTableImportException ex)
        {
            await ShowMessageAsync($"The truth table could not be imported.\n{ex.Message}");
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"The truth table file could not be opened.\n{ex.Message}");
        }
    }

    private void MainWindow_OnKeyDown(object? sender, KeyEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsGateDiagramVisible: true })
        {
            return;
        }

        if (e.Key == Key.Enter && e.KeyModifiers == KeyModifiers.None)
        {
            _ = SubmitGateDiagramEditingAsync();
            e.Handled = true;
            return;
        }

        if (e.Key != Key.Escape)
        {
            return;
        }

        CancelGateDiagramEditing();
        e.Handled = true;
    }

    private void GateDeleteSelected_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            GateDiagramEditor.DeleteSelected();
        }
    }

    private async void GateSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            await SubmitGateDiagramEditingAsync();
        }
    }

    private void GateCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsGateDiagramVisible: true })
        {
            CancelGateDiagramEditing();
        }
    }

    private async Task SubmitGateDiagramEditingAsync()
    {
        GateDiagramEditor.ClearActivePaletteButton();
        GateDiagramEditor.CancelInteraction();

        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        if (!viewModel.SubmitGateDiagramEditing(out var errorMessage))
        {
            if (!string.IsNullOrWhiteSpace(errorMessage))
            {
                await ShowMessageAsync(errorMessage, "Diagram Error");
            }

            return;
        }

        if (viewModel.GetSelectedFunction() is { } logicFunction)
        {
            FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        }
    }

    private void CancelGateDiagramEditing()
    {
        GateDiagramEditor.ClearActivePaletteButton();
        GateDiagramEditor.CancelInteraction();

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelGateDiagramEditing();
        }
    }

    private async void GateDiagramEditor_OnSubmitRequested(object? sender, EventArgs e)
    {
        await SubmitGateDiagramEditingAsync();
    }

    private void GateDiagramEditor_OnCancelRequested(object? sender, EventArgs e)
    {
        CancelGateDiagramEditing();
    }

    private async void GateDiagramEditor_OnHelpRequested(object? sender, EventArgs e)
    {
        await OpenUrlAsync(GateDiagramHelpUrl, "Gate diagram help could not be opened.");
    }

    private async void GateDiagramEditor_OnVariableNameRequested(
        object? sender,
        GateDiagramVariableNameRequestedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        var dialog = new GateVariableNameDialog();
        dialog.ViewModel.ExistingNames = viewModel.GateDiagramItems
                .Where(static item => item.Kind is GatePaletteKind.Input or GatePaletteKind.Output)
                .Select(static item => item.Label);
        dialog.ViewModel.VariableName = GetProposedGateVariableName(e.Item.Kind, viewModel.GateDiagramItems);

        var result = await dialog.ShowDialog<bool?>(this);
        if (result == true)
        {
            e.AddItem(dialog.ViewModel.VariableName);
        }
    }

    private static string GetProposedGateVariableName(
        GatePaletteKind kind,
        IEnumerable<GateDiagramItem> items)
    {
        var existingNames = items
            .Where(static item => item.Kind is GatePaletteKind.Input or GatePaletteKind.Output)
            .Select(static item => item.Label)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        if (kind == GatePaletteKind.Output && !existingNames.Contains("X"))
        {
            return "X";
        }

        const string alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        foreach (var name in alphabet.Select(static character => character.ToString()))
        {
            if (!existingNames.Contains(name))
            {
                return name;
            }
        }

        for (var index = 0; index < 100; index++)
        {
            var name = $"V{index}";
            if (!existingNames.Contains(name))
            {
                return name;
            }
        }

        return string.Empty;
    }

    private async Task OpenUrlAsync(string url, string errorMessage)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"{errorMessage}\n{ex.Message}");
        }
    }

    private void CloseDocument_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CloseCurrentDocument();
            TruthTableEditor.ClearColumns();
            FunctionTruthTableView.ClearColumns();
        }
    }

    private async void OpenDocument_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsFileOpenEnabled: true } viewModel)
        {
            return;
        }

        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Open Logic Function",
            AllowMultiple = false,
            FileTypeFilter =
            [
                new FilePickerFileType("Logic Function Files")
                {
                    Patterns = [ "*.lfcn" ]
                },
                new FilePickerFileType("All Files")
                {
                    Patterns = [ "*" ]
                }
            ]
        });

        var file = files.FirstOrDefault();
        if (file is null)
        {
            return;
        }

        if (viewModel.OpenFunction(file.Path.LocalPath))
        {
            ConfigureSelectedFunctionTruthTableColumns(viewModel);
            return;
        }

        await ShowMessageAsync(viewModel.StatusText, "Open Logic Function");
    }

    private async void Save_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsFileSaveEnabled: true } viewModel)
        {
            return;
        }

        if (viewModel.SaveSelectedFunction())
        {
            return;
        }

        if (viewModel.StatusText == "Save As required")
        {
            await SaveSelectedFunctionAsAsync(viewModel);
            return;
        }

        await ShowMessageAsync(viewModel.StatusText, "Save Logic Function");
    }

    private async void SaveAs_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsFileSaveAsEnabled: true } viewModel)
        {
            await SaveSelectedFunctionAsAsync(viewModel);
        }
    }

    private async void ExportTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsFileExportTruthTableEnabled: true } viewModel)
        {
            return;
        }

        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Truth Table",
            SuggestedFileName = GetSuggestedFileName(viewModel, ".csv"),
            DefaultExtension = "csv",
            FileTypeChoices =
            [
                new FilePickerFileType("Comma Separated Values")
                {
                    Patterns = [ "*.csv" ]
                },
                new FilePickerFileType("All Files")
                {
                    Patterns = [ "*" ]
                }
            ]
        });

        if (file is null)
        {
            return;
        }

        var csv = viewModel.ExportSelectedTruthTableCsv();
        if (csv is null)
        {
            await ShowMessageAsync(viewModel.StatusText, "Export Truth Table");
            return;
        }

        try
        {
            await using var output = await file.OpenWriteAsync();
            if (output.CanSeek)
            {
                output.SetLength(0);
            }

            await using var writer = new StreamWriter(output);
            await writer.WriteAsync(csv);
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"The truth table could not be exported.\n{ex.Message}", "Export Truth Table");
        }
    }

    private async void ExportGateDiagram_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsFileExportGateDiagramEnabled: true } viewModel)
        {
            return;
        }

        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Export Gate Diagram",
            SuggestedFileName = GetSuggestedFileName(viewModel, ".svg"),
            DefaultExtension = "svg",
            FileTypeChoices =
            [
                new FilePickerFileType("Scalable Vector Graphics")
                {
                    Patterns = [ "*.svg" ]
                },
                new FilePickerFileType("All Files")
                {
                    Patterns = [ "*" ]
                }
            ]
        });

        if (file is null)
        {
            return;
        }

        var svg = viewModel.ExportSelectedGateDiagramSvg();
        if (svg is null)
        {
            await ShowMessageAsync(viewModel.StatusText, "Export Gate Diagram");
            return;
        }

        try
        {
            await using var output = await file.OpenWriteAsync();
            if (output.CanSeek)
            {
                output.SetLength(0);
            }

            await using var writer = new StreamWriter(output);
            await writer.WriteAsync(svg);
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"The gate diagram could not be exported.\n{ex.Message}", "Export Gate Diagram");
        }
    }

    private async void Print_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel { IsFilePrintEnabled: true } viewModel)
        {
            viewModel.PrintSelectedFunction();
            await ShowMessageAsync(viewModel.StatusText, "Print");
        }
    }

    private async Task SaveSelectedFunctionAsAsync(MainWindowViewModel viewModel)
    {
        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = "Save Logic Function",
            SuggestedFileName = GetSuggestedFileName(viewModel, ".lfcn"),
            DefaultExtension = "lfcn",
            FileTypeChoices =
            [
                new FilePickerFileType("Logic Function Files")
                {
                    Patterns = [ "*.lfcn" ]
                },
                new FilePickerFileType("All Files")
                {
                    Patterns = [ "*" ]
                }
            ]
        });

        if (file is null)
        {
            return;
        }

        if (!viewModel.SaveSelectedFunctionAs(file.Path.LocalPath))
        {
            await ShowMessageAsync(viewModel.StatusText, "Save Logic Function");
        }
    }

    private static string GetSuggestedFileName(MainWindowViewModel viewModel, string extension)
    {
        var selectedFunction = viewModel.GetSelectedFunction();
        var baseName = selectedFunction?.OutputNames.Length switch
        {
            1 => selectedFunction.OutputNames[0],
            > 1 => $"{selectedFunction.OutputNames[0]}-{selectedFunction.OutputNames[^1]}",
            _ => "function"
        };

        return string.Concat(baseName, extension);
    }

    private void Exit_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private void FunctionsGrid_OnSelectionChanged(object? sender, EventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
        {
            return;
        }

        viewModel.SetSelectedFunctionSummaries(FunctionsGrid.SelectedSummaries);
        if (FunctionsGrid.SelectedItemCount != 1 ||
            viewModel.GetSelectedFunction() is not { } logicFunction)
        {
            return;
        }

        FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
        viewModel.ShowFunction(logicFunction);
    }

    private void TruthTableEditor_OnSelectionChanged(object? sender, EventArgs e)
    {
        UpdateTruthTableMainMenuState();
    }

    private void TruthTableEditor_OnSubmitRequested(object? sender, EventArgs e)
    {
        SubmitTruthTableEditing();
    }

    private void TruthTableEditor_OnCancelRequested(object? sender, EventArgs e)
    {
        CancelTruthTableEditing();
    }

    private void UpdateTruthTableMainMenuState()
    {
        var isTruthTableVisible = DataContext is MainWindowViewModel { IsTruthTableVisible: true };
        var hasSelectedRows =
            isTruthTableVisible &&
            TruthTableEditor.HasSelectedRows;

        TruthTableSelectAllMainMenuItem.IsEnabled = isTruthTableVisible;
        TruthTableSetTrueMainMenuItem.IsEnabled = hasSelectedRows;
        TruthTableSetFalseMainMenuItem.IsEnabled = hasSelectedRows;
        TruthTableSetDontCareMainMenuItem.IsEnabled = hasSelectedRows;
        TruthTableInvertMainMenuItem.IsEnabled = hasSelectedRows;
    }

    private async void RefreshEquationEditorToolbarButtons()
    {
        var isEditingEquation = DataContext is MainWindowViewModel { IsEquationEditorVisible: true };
        var hasSelection = isEditingEquation && LogicEquationEditor.HasSelection;

        EquationEditorCutToolbarButton.IsEnabled = hasSelection;
        EquationEditorCopyToolbarButton.IsEnabled = hasSelection;
        EquationEditorUndoToolbarButton.IsEnabled = isEditingEquation && LogicEquationEditor.CanUndo;
        EquationEditorRedoToolbarButton.IsEnabled = isEditingEquation && LogicEquationEditor.CanRedo;
        EquationEditorPasteToolbarButton.IsEnabled = false;
        EquationCutMainMenuItem.IsEnabled = hasSelection;
        EquationCopyMainMenuItem.IsEnabled = isEditingEquation && hasSelection;
        EquationUndoMainMenuItem.IsEnabled = isEditingEquation && LogicEquationEditor.CanUndo;
        EquationRedoMainMenuItem.IsEnabled = isEditingEquation && LogicEquationEditor.CanRedo;
        EquationPasteMainMenuItem.IsEnabled = false;
        EquationDeleteMainMenuItem.IsEnabled = isEditingEquation && hasSelection;
        EquationSelectAllMainMenuItem.IsEnabled = isEditingEquation && LogicEquationEditor.HasText;

        if (!isEditingEquation)
        {
            return;
        }

        var canPaste = await LogicEquationEditor.CanPasteAsync();
        EquationEditorPasteToolbarButton.IsEnabled = canPaste;
        EquationPasteMainMenuItem.IsEnabled = canPaste;
    }

    private void EquationEditorUndo_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.Undo();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorRedo_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.Redo();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorCut_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.Cut();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorCopy_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.Copy();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorPaste_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.Paste();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorDelete_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.DeleteSelection();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        LogicEquationEditor.FocusEditor();
        LogicEquationEditor.SelectAllText();
        RefreshEquationEditorToolbarButtons();
    }

    private void EquationEditorSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitLogicEquationEditing();
    }

    private void SubmitLogicEquationEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SubmitLogicEquationEditing();
            if (viewModel.GetSelectedFunction() is { } logicFunction)
            {
                FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
            }
        }
    }

    private void EquationEditorCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        CancelLogicEquationEditing();
    }

    private void CancelLogicEquationEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelLogicEquationEditing();
            RefreshEquationEditorToolbarButtons();
        }
    }

    private void LogicEquationEditor_OnSubmitRequested(object? sender, EventArgs e)
    {
        SubmitLogicEquationEditing();
    }

    private void LogicEquationEditor_OnCancelRequested(object? sender, EventArgs e)
    {
        CancelLogicEquationEditing();
    }

    private void LogicEquationEditor_OnStateChanged(object? sender, EventArgs e)
    {
        RefreshEquationEditorToolbarButtons();
    }

    private void TruthTableSelectAll_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { IsTruthTableVisible: true })
        {
            return;
        }

        TruthTableEditor.SelectAllEditableRows();
        UpdateTruthTableMainMenuState();
    }

    private void TruthTableSetTrue_OnClick(object? sender, RoutedEventArgs e)
    {
        TruthTableEditor.SetContextRowsOutputValues("1");
    }

    private void TruthTableSetFalse_OnClick(object? sender, RoutedEventArgs e)
    {
        TruthTableEditor.SetContextRowsOutputValues("0");
    }

    private void TruthTableSetDontCare_OnClick(object? sender, RoutedEventArgs e)
    {
        TruthTableEditor.SetContextRowsOutputValues("X");
    }

    private void TruthTableInvert_OnClick(object? sender, RoutedEventArgs e)
    {
        TruthTableEditor.InvertContextRows();
    }

    private void TruthTableSubmit_OnClick(object? sender, RoutedEventArgs e)
    {
        SubmitTruthTableEditing();
    }

    private void TruthTableCancel_OnClick(object? sender, RoutedEventArgs e)
    {
        CancelTruthTableEditing();
    }

    private void SubmitTruthTableEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.SubmitTruthTableEditing();
            if (viewModel.GetSelectedFunction() is { } logicFunction)
            {
                FunctionTruthTableView.ConfigureColumns(logicFunction.InputNames, logicFunction.OutputNames);
            }

            TruthTableEditor.ClearColumns();
            UpdateTruthTableMainMenuState();
        }
    }

    private void CancelTruthTableEditing()
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.CancelTruthTableEditing();
            TruthTableEditor.ClearColumns();
            UpdateTruthTableMainMenuState();
        }
    }

    private async Task ShowMessageAsync(string message)
    {
        await ShowMessageAsync(message, "Logic Friday");
    }

    private async Task ShowMessageAsync(string message, string title)
    {
        var dialog = new Window
        {
            Title = title,
            Width = 380,
            Height = 150,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            CanResize = false
        };

        var okButton = new Button
        {
            Content = "OK",
            MinWidth = 80,
            HorizontalAlignment = HorizontalAlignment.Right,
            IsDefault = true
        };

        okButton.Click += (_, _) => dialog.Close();

        dialog.Content = new Grid
        {
            RowDefinitions = new RowDefinitions("*,Auto"),
            Margin = new Avalonia.Thickness(16),
            Children =
            {
                new TextBlock
                {
                    Text = message,
                    TextWrapping = TextWrapping.Wrap
                },
                okButton
            }
        };

        Grid.SetRow(okButton, 1);
        await dialog.ShowDialog(this);
    }

    private async Task ShowTextDialogAsync(string text, string title)
    {
        var dialog = new Window
        {
            Title = title,
            Width = 420,
            Height = 320,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            CanResize = true
        };

        var okButton = new Button
        {
            Content = "OK",
            MinWidth = 80,
            HorizontalAlignment = HorizontalAlignment.Right,
            IsDefault = true
        };

        okButton.Click += (_, _) => dialog.Close();

        dialog.Content = new Grid
        {
            RowDefinitions = new RowDefinitions("*,Auto"),
            Margin = new Avalonia.Thickness(16),
            RowSpacing = 8,
            Children =
            {
                new TextBox
                {
                    Text = text,
                    IsReadOnly = true,
                    AcceptsReturn = true,
                    TextWrapping = TextWrapping.NoWrap,
                    FontFamily = FontFamily.Parse("Consolas")
                },
                okButton
            }
        };

        Grid.SetRow(okButton, 1);
        await dialog.ShowDialog(this);
    }
}
