using Avalonia.Controls;
using Avalonia.Interactivity;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class TruthTableSetupDialog : Window
{
    public TruthTableSetupDialog()
    {
        InitializeComponent();
        ViewModel = new TruthTableSetupDialogViewModel();
        DataContext = ViewModel;
    }

    public TruthTableSetupDialogViewModel ViewModel { get; }

    private void InputCount_OnValueChanged(object? sender, NumericUpDownValueChangedEventArgs e)
    {
        ViewModel.SetInputCount((int)(e.NewValue ?? ViewModel.InputCount));
    }

    private void OutputCount_OnValueChanged(object? sender, NumericUpDownValueChangedEventArgs e)
    {
        ViewModel.SetOutputCount((int)(e.NewValue ?? ViewModel.OutputCount));
    }

    private async void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        var validationError = ViewModel.ValidateNames();
        if (validationError is not null)
        {
            await ShowMessageAsync(validationError);
            return;
        }

        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }

    private async void HelpButton_OnClick(object? sender, RoutedEventArgs e)
    {
        await ShowMessageAsync("Help contents are not available yet.");
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
            HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Right,
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
                    TextWrapping = Avalonia.Media.TextWrapping.Wrap
                },
                okButton
            }
        };

        Grid.SetRow(okButton, 1);
        await dialog.ShowDialog(this);
    }

}
