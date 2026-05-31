using Avalonia.Controls;
using Avalonia.Interactivity;
using LogicFriday1.ViewModels;

namespace LogicFriday1.Views;

public partial class MinimizeDialog : Window
{
    public MinimizeDialog()
    {
        InitializeComponent();
        ViewModel = new MinimizeDialogViewModel();
        DataContext = ViewModel;
    }

    public MinimizeDialogViewModel ViewModel
    {
        get;
    }

    public int OutputCount
    {
        get => ViewModel.OutputCount;
        set => ViewModel.OutputCount = value;
    }

    private void FastMode_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectFastMode();
    }

    private void ExactMode_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectExactMode();
    }

    private void IndependentOutputs_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectIndependentOutputs();
    }

    private void JointOutputs_OnClick(object? sender, RoutedEventArgs e)
    {
        ViewModel.SelectJointOutputs();
    }

    private void OkButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(true);
    }

    private void CancelButton_OnClick(object? sender, RoutedEventArgs e)
    {
        Close(false);
    }
}
