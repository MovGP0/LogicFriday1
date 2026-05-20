using Avalonia.Controls;
using Avalonia.Interactivity;

namespace LogicFriday1.Views;

public partial class AboutDialog : Window
{
    public AboutDialog()
    {
        InitializeComponent();
    }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }
}
