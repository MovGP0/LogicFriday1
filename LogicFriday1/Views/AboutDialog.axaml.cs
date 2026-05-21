using System;
using System.Diagnostics;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;

namespace LogicFriday1.Views;

public partial class AboutDialog : Window
{
    private const string EmailUri = "mailto:logic.friday@sontrak.com";
    private const string WebsiteUri = "http://www.sontrak.com";

    public AboutDialog()
    {
        InitializeComponent();
    }

    private async void EmailButton_OnClick(object? sender, RoutedEventArgs e)
    {
        await OpenExternalTargetAsync(EmailUri, "No registered mail application could be found.");
    }

    private async void WebsiteButton_OnClick(object? sender, RoutedEventArgs e)
    {
        await OpenExternalTargetAsync(WebsiteUri, "No registered web browser could be found.");
    }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private async Task OpenExternalTargetAsync(string target, string failureMessage)
    {
        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = target,
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            await ShowMessageAsync(failureMessage);
        }
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
