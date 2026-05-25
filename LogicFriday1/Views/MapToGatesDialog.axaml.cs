using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;

namespace LogicFriday1.Views;

public partial class MapToGatesDialog : Window
{
    public MapToGatesDialog()
    {
        InitializeComponent();
        DataContext = this;
    }

    public bool UseInverter { get; set; } = true;

    public bool UseNand2 { get; set; } = true;

    public bool UseNand3 { get; set; }

    public bool UseNand4 { get; set; }

    public bool UseNor2 { get; set; } = true;

    public bool UseNor3 { get; set; }

    public bool UseNor4 { get; set; }

    public bool UseXor2 { get; set; }

    public bool UseMux2 { get; set; }

    public bool UseAnd2 { get; set; }

    public bool UseAnd3 { get; set; }

    public bool UseAnd4 { get; set; }

    public bool UseOr2 { get; set; }

    public bool UseOr3 { get; set; }

    public bool UseOr4 { get; set; }

    public bool UseStandardLogicIcs { get; set; } = true;

    public bool UseDieArea { get; set; }

    private void StandardLogicToggle_OnClick(object? sender, RoutedEventArgs e)
    {
        UseStandardLogicIcs = true;
        UseDieArea = false;
        StandardLogicToggle.IsChecked = true;
        DieAreaToggle.IsChecked = false;
    }

    private void DieAreaToggle_OnClick(object? sender, RoutedEventArgs e)
    {
        UseStandardLogicIcs = false;
        UseDieArea = true;
        StandardLogicToggle.IsChecked = false;
        DieAreaToggle.IsChecked = true;
    }

    private async void HelpButton_OnClick(object? sender, RoutedEventArgs e)
    {
        await ShowMessageAsync("Map to Gates help is tracked by LogicFriday1-8j8.2.2.");
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
