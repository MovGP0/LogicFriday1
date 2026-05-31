using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Services;

namespace LogicFriday1.ViewModels;

public partial class MinimizeDialogViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _useFastMode = true;

    [ObservableProperty]
    private bool _useExactMode;

    [ObservableProperty]
    private bool _minimizeOutputsIndependently = true;

    [ObservableProperty]
    private bool _minimizeOutputsJointly;

    public MinimizeDialogViewModel(int outputCount)
    {
        CanChooseMultipleOutputMode = outputCount > 1;
    }

    public bool CanChooseMultipleOutputMode
    {
        get;
    }

    public void SelectFastMode()
    {
        UseFastMode = true;
        UseExactMode = false;
    }

    public void SelectExactMode()
    {
        UseFastMode = false;
        UseExactMode = true;
    }

    public void SelectIndependentOutputs()
    {
        MinimizeOutputsIndependently = true;
        MinimizeOutputsJointly = false;
    }

    public void SelectJointOutputs()
    {
        if (!CanChooseMultipleOutputMode)
        {
            SelectIndependentOutputs();
            return;
        }

        MinimizeOutputsIndependently = false;
        MinimizeOutputsJointly = true;
    }

    public MinimizeOptions ToMinimizeOptions()
    {
        return new MinimizeOptions(
            UseExactMode,
            MinimizeOutputsIndependently || !CanChooseMultipleOutputMode);
    }
}
