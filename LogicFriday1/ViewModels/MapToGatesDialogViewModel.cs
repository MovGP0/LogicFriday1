using System.Text;
using CommunityToolkit.Mvvm.ComponentModel;
using LogicFriday1.Sis;

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

    public bool HasRequiredGateFamily()
    {
        return UseNand2 ||
            UseNand3 ||
            UseNand4 ||
            UseNor2 ||
            UseNor3 ||
            UseNor4;
    }

    public SisMapOptions ToSisMapOptions()
    {
        return new SisMapOptions(
            InvertOutputs: false,
            ReadLibraryNoDecomp: false,
            MapMode: UseDieArea ? SisMapMode.M1 : SisMapMode.Default);
    }

    public string BuildGenlib()
    {
        var builder = new StringBuilder()
            .AppendLine("GATE zero 0 O=CONST0;")
            .AppendLine("GATE one 0 O=CONST1;");

        if (UseInverter)
        {
            builder.AppendLine("GATE inv 1 O=!a;");
        }

        AppendGate(builder, UseNand2, "nand2", 2, "!(" + Product("a", "b") + ")");
        AppendGate(builder, UseNand3, "nand3", 3, "!(" + Product("a", "b", "c") + ")");
        AppendGate(builder, UseNand4, "nand4", 4, "!(" + Product("a", "b", "c", "d") + ")");
        AppendGate(builder, UseNor2, "nor2", 2, "!(" + Sum("a", "b") + ")");
        AppendGate(builder, UseNor3, "nor3", 3, "!(" + Sum("a", "b", "c") + ")");
        AppendGate(builder, UseNor4, "nor4", 4, "!(" + Sum("a", "b", "c", "d") + ")");
        AppendGate(builder, UseXor2, "xor2", 2, "a^b");
        AppendGate(builder, UseMux2, "mux2", 3, "a*!s+b*s");
        AppendGate(builder, UseAnd2, "and2", 2, Product("a", "b"));
        AppendGate(builder, UseAnd3, "and3", 3, Product("a", "b", "c"));
        AppendGate(builder, UseAnd4, "and4", 4, Product("a", "b", "c", "d"));
        AppendGate(builder, UseOr2, "or2", 2, Sum("a", "b"));
        AppendGate(builder, UseOr3, "or3", 3, Sum("a", "b", "c"));
        AppendGate(builder, UseOr4, "or4", 4, Sum("a", "b", "c", "d"));

        return builder.ToString();
    }

    private static void AppendGate(
        StringBuilder builder,
        bool enabled,
        string name,
        int area,
        string expression)
    {
        if (enabled)
        {
            builder.AppendLine($"GATE {name} {area} O={expression};");
        }
    }

    private static string Product(params string[] inputs)
    {
        return string.Join("*", inputs);
    }

    private static string Sum(params string[] inputs)
    {
        return string.Join("+", inputs);
    }
}
