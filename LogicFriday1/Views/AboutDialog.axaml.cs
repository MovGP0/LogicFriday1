using System;
using System.ComponentModel;
using System.Diagnostics;
using Avalonia.Controls;
using Avalonia.Interactivity;

namespace LogicFriday1.Views;

public partial class AboutDialog : Window
{
    private const string EmailUri = "mailto:logic.friday@sontrak.com";
    private const string WebsiteUri = "http://www.sontrak.com";

    public AboutDialog()
    {
        InitializeComponent();
    }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private void EmailButton_OnClick(object? sender, RoutedEventArgs e)
    {
        OpenUri(EmailUri, "Could not find a registered email application.");
    }

    private void WebsiteButton_OnClick(object? sender, RoutedEventArgs e)
    {
        OpenUri(WebsiteUri, "Could not find a registered web browser.");
    }

    private async void OpenUri(string uri, string failureMessage)
    {
        try
        {
            Process.Start(new ProcessStartInfo(uri)
            {
                UseShellExecute = true
            });
        }
        catch (Exception exception) when (exception is Win32Exception or InvalidOperationException)
        {
            var messageBox = new Window
            {
                Title = "Logic Friday",
                Width = 360,
                Height = 140,
                WindowStartupLocation = WindowStartupLocation.CenterOwner,
                CanResize = false,
                Content = new Grid
                {
                    RowDefinitions = new RowDefinitions("*,Auto"),
                    Margin = new Avalonia.Thickness(16),
                    Children =
                    {
                        new TextBlock
                        {
                            Text = failureMessage,
                            TextWrapping = Avalonia.Media.TextWrapping.Wrap
                        },
                        new Button
                        {
                            Content = "OK",
                            MinWidth = 80,
                            HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Right,
                            VerticalAlignment = Avalonia.Layout.VerticalAlignment.Bottom
                        }
                    }
                }
            };

            if (messageBox.Content is Grid grid && grid.Children[1] is Button button)
            {
                Grid.SetRow(button, 1);
                button.Click += (_, _) => messageBox.Close();
            }

            await messageBox.ShowDialog(this);
        }
    }
}
