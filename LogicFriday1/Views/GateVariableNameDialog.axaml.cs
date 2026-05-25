using Avalonia.Controls;
using Avalonia.Interactivity;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class GateVariableNameDialog : Window
{
    public GateVariableNameDialog()
    {
        InitializeComponent();
        ViewModel = new GateVariableNameDialogViewModel();
        DataContext = ViewModel;

        Opened += (_, _) =>
        {
            VariableNameTextBox.Focus();
            VariableNameTextBox.SelectAll();
        };
    }

    public GateVariableNameDialogViewModel ViewModel { get; }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        if (!ViewModel.TryAccept())
        {
            return;
        }

        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }
}
