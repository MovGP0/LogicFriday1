using CommunityToolkit.Mvvm.ComponentModel;

namespace LogicFriday1.ViewModels;

public partial class MapToGatesDialogViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _useInverter = true;

    [ObservableProperty]
    private bool _useNand2 = true;

    [ObservableProperty]
    private bool _useNand3;

    [ObservableProperty]
    private bool _useNand4;

    [ObservableProperty]
    private bool _useNor2 = true;

    [ObservableProperty]
    private bool _useNor3;

    [ObservableProperty]
    private bool _useNor4;

    [ObservableProperty]
    private bool _useXor2;

    [ObservableProperty]
    private bool _useMux2;

    [ObservableProperty]
    private bool _useAnd2;

    [ObservableProperty]
    private bool _useAnd3;

    [ObservableProperty]
    private bool _useAnd4;

    [ObservableProperty]
    private bool _useOr2;

    [ObservableProperty]
    private bool _useOr3;

    [ObservableProperty]
    private bool _useOr4;

    [ObservableProperty]
    private bool _useStandardLogicIcs = true;

    [ObservableProperty]
    private bool _useDieArea;

    public void SelectStandardLogicIcs()
    {
        UseStandardLogicIcs = true;
        UseDieArea = false;
    }

    public void SelectDieArea()
    {
        UseStandardLogicIcs = false;
        UseDieArea = true;
    }
}
