using System;
using System.Diagnostics;
using System.IO;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MainWindow : Window
{
    private const string HelpFileName = "lf.chm";
    private const string HelpContentsTopic = "features.htm";

    public MainWindow()
    {
        InitializeComponent();
    }

    private async void HelpContents_OnClick(object? sender, RoutedEventArgs e)
    {
        if (!OperatingSystem.IsWindows())
        {
            await ShowMessageAsync("Help contents require Windows HTML Help.");
            return;
        }

        var helpFilePath = FindHelpFilePath();
        if (helpFilePath is null)
        {
            await ShowMessageAsync("Help contents are not available because lf.chm was not found.");
            return;
        }

        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = "hh.exe",
                Arguments = $"\"{helpFilePath}::/{HelpContentsTopic}\"",
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            await ShowMessageAsync($"Help contents could not be opened.\n{ex.Message}");
        }
    }

    private async void AboutLogicFriday_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new AboutDialog();
        await dialog.ShowDialog(this);
    }

    private void MinimizedView_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ShowMinimizedView();
        }
    }

    private async void NewTruthTable_OnClick(object? sender, RoutedEventArgs e)
    {
        var dialog = new TruthTableSetupDialog();
        await dialog.ShowDialog<bool?>(this);
    }

    private void NewLogicEquation_OnClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.StartNewLogicEquation();
            EquationEditor.Focus();
        }
    }

    private void Exit_OnClick(object? sender, RoutedEventArgs e)
    {
        Close();
    }

    private static string? FindHelpFilePath()
    {
        var candidateDirectories = new[]
        {
            AppContext.BaseDirectory,
            Environment.CurrentDirectory,
            Path.Combine(AppContext.BaseDirectory, "Help")
        };

        foreach (var directory in candidateDirectories)
        {
            var helpFilePath = Path.Combine(directory, HelpFileName);
            if (File.Exists(helpFilePath))
            {
                return helpFilePath;
            }
        }

        return null;
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
