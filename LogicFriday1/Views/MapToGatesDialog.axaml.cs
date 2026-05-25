using System.Diagnostics;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MapToGatesDialog : Window
{
    private const string MappingHelpUrl = "https://github.com/MovGP0/LogicFriday1/wiki/Mapping-a-function-to-a-gate-diagram";

    public MapToGatesDialog()
    {
        InitializeComponent();
        ViewModel = new MapToGatesDialogViewModel();
        DataContext = ViewModel;
    }

    public MapToGatesDialogViewModel ViewModel { get; }

    private void StandardLogicToggle_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectStandardLogicIcs();
    }

    private void DieAreaToggle_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectDieArea();
    }

    private async void HelpButton_OnClick(object? sender, RoutedEventArgs e)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = MappingHelpUrl,
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"Map to Gates help could not be opened.\n{ex.Message}");
        }
    }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }

    private async Task ShowMessageAsync(string message)
    {
        var dialog = new Window
        {
            Title = "Logic Friday",
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
}
