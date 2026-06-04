using LogicFriday1.ViewModels;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public void StartModifyLogicEquation_LoadsSelectedEquationText()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        viewModel.StartModifyLogicEquation();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsEquationEditorVisible.ShouldBeTrue(),
            static vm => vm.LogicEquationText.ShouldBe("F = A;"));
    }

    [Fact]
    public void SubmitLogicEquationEditing_WhenModifying_ReplacesSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");
        var originalSummary = viewModel.SelectedFunctionSummary;

        viewModel.StartModifyLogicEquation();
        viewModel.LogicEquationText = "F = A';";
        viewModel.SubmitLogicEquationEditing();

        viewModel.ShouldSatisfyAllConditions(
            vm => vm.FunctionSummaries.Count.ShouldBe(1),
            vm => vm.SelectedFunctionSummary.ShouldNotBe(originalSummary),
            static vm => vm.GetSelectedFunction()!.EquationText.ShouldBe("F = A';"),
            static vm => vm.IsEquationEditorVisible.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Logic equation modified"));
    }

    [Fact]
    public void CancelLogicEquationEditing_WhenModifying_PreservesSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");
        var originalFunction = viewModel.GetSelectedFunction();

        viewModel.StartModifyLogicEquation();
        viewModel.LogicEquationText = "F = A';";
        viewModel.CancelLogicEquationEditing();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.FunctionSummaries.Count.ShouldBe(1),
            vm => vm.GetSelectedFunction().ShouldBeSameAs(originalFunction),
            static vm => vm.LogicEquationText.ShouldBe("F = A;"),
            static vm => vm.IsEquationEditorVisible.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Ready"));
    }

    [Fact]
    public void ShowSumOfProductsEquation_GeneratesSumOfProductsText()
    {
        var viewModel = CreateXorTruthTableFunction();

        viewModel.ShowSumOfProductsEquation();

        viewModel.LogicEquationText.ShouldBe(string.Join(
            Environment.NewLine,
            "Sum of Products:",
            "F = A' B + A B';"));
    }

    [Fact]
    public void ShowProductOfSumsEquation_GeneratesProductOfSumsText()
    {
        var viewModel = CreateXorTruthTableFunction();

        viewModel.ShowProductOfSumsEquation();

        viewModel.LogicEquationText.ShouldBe(string.Join(
            Environment.NewLine,
            "Product of Sums:",
            "F = (A + B) (A' + B');"));
    }

    [Fact]
    public void FactorSelectedEquation_GeneratesLocalFactoredText()
    {
        var viewModel = CreateXorTruthTableFunction();

        viewModel.FactorSelectedEquation();

        viewModel.LogicEquationText.ShouldBe(string.Join(
            Environment.NewLine,
            "Factored:",
            "Sum of Products:",
            "F = A' B + A B';"));
    }

    [Fact]
    public void EquationCommandEnablement_IsDisabledWithoutSelection()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsEquationModifyEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationFormatEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationSubmitEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationCancelEnabled.ShouldBeFalse());
    }

    [Fact]
    public void EquationCommandEnablement_IsEnabledForSingleSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();

        AddEquation(viewModel, "F = A;");

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsEquationModifyEnabled.ShouldBeTrue(),
            static vm => vm.IsEquationFormatEnabled.ShouldBeTrue());
    }

    [Fact]
    public void EquationCommandEnablement_SwitchesToSubmitAndCancelWhileEditing()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        viewModel.StartModifyLogicEquation();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsEquationModifyEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationFormatEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationSubmitEnabled.ShouldBeTrue(),
            static vm => vm.IsEquationCancelEnabled.ShouldBeTrue());
    }

    [Fact]
    public void EquationCommandEnablement_IsDisabledForMultipleSelectedFunctions()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        viewModel.CancelLogicEquationEditing();
        viewModel.SetSelectedFunctionCount(2);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsEquationModifyEnabled.ShouldBeFalse(),
            static vm => vm.IsEquationFormatEnabled.ShouldBeFalse());
    }

    [Fact]
    public void TruthTableShowMode_DefaultsToTrueAndDontCareRows()
    {
        var viewModel = CreateTruthTableShowModeViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsTruthTableShowModeEnabled.ShouldBeTrue(),
            static vm => vm.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue(),
            static vm => GetTerms(vm).ShouldBe(["1", "2", "3"]));
    }

    [Fact]
    public void ShowAllTruthTableRows_ShowsAllTerms()
    {
        var viewModel = CreateTruthTableShowModeViewModel();

        viewModel.ShowAllTruthTableRows();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsShowAllTruthTableRowsSelected.ShouldBeTrue(),
            static vm => GetTerms(vm).ShouldBe(["0", "1", "2", "3"]));
    }

    [Fact]
    public void ShowTrueAndDontCareTruthTableRows_RestoresFilteredTerms()
    {
        var viewModel = CreateTruthTableShowModeViewModel();

        viewModel.ShowAllTruthTableRows();
        viewModel.ShowTrueAndDontCareTruthTableRows();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue(),
            static vm => GetTerms(vm).ShouldBe(["1", "2", "3"]));
    }

    [Fact]
    public void TruthTableShowMode_NewFunctionDefaultsToTrueAndDontCareRows()
    {
        var viewModel = new MainWindowViewModel();
        viewModel.StartImportedTruthTable(
            ["A"],
            ["F"],
            [
                ["0"],
                ["1"]
            ]);
        viewModel.SubmitTruthTableEditing();
        var firstSummary = viewModel.SelectedFunctionSummary;

        viewModel.ShowAllTruthTableRows();

        viewModel.StartImportedTruthTable(
            ["B"],
            ["G"],
            [
                ["0"],
                ["1"]
            ]);
        viewModel.SubmitTruthTableEditing();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue(),
            static vm => GetTerms(vm).ShouldBe(["1"]));
    }

    [Fact]
    public void TruthTableShowMode_RestoresSelectionWhenFunctionIsReselected()
    {
        var viewModel = new MainWindowViewModel();
        viewModel.StartImportedTruthTable(
            ["A"],
            ["F"],
            [
                ["0"],
                ["1"]
            ]);
        viewModel.SubmitTruthTableEditing();
        var firstSummary = viewModel.SelectedFunctionSummary;

        viewModel.ShowAllTruthTableRows();

        viewModel.StartImportedTruthTable(
            ["B"],
            ["G"],
            [
                ["0"],
                ["1"]
            ]);
        viewModel.SubmitTruthTableEditing();

        viewModel.SelectedFunctionSummary = firstSummary;
        viewModel.ShowFunction(firstSummary!.LogicFunction!);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsShowAllTruthTableRowsSelected.ShouldBeTrue(),
            static vm => GetTerms(vm).ShouldBe(["0", "1"]));
    }

    private static string[] GetTerms(MainWindowViewModel viewModel)
    {
        return viewModel.FunctionTruthTableRows
            .Select(static row => row.Cells[0].Value)
            .ToArray();
    }

    private static void AddEquation(MainWindowViewModel viewModel, string equationText)
    {
        viewModel.StartNewLogicEquation();
        viewModel.LogicEquationText = equationText;
        viewModel.SubmitLogicEquationEditing();
    }

    private static MainWindowViewModel CreateTruthTableShowModeViewModel()
    {
        var viewModel = new MainWindowViewModel();
        viewModel.StartImportedTruthTable(
            ["A", "B"],
            ["F", "G"],
            [
                ["0", "0"],
                ["1", "0"],
                ["X", "0"],
                ["0", "1"]
            ]);
        viewModel.SubmitTruthTableEditing();

        return viewModel;
    }

    private static MainWindowViewModel CreateXorTruthTableFunction()
    {
        var viewModel = new MainWindowViewModel();
        viewModel.StartImportedTruthTable(
            ["A", "B"],
            ["F"],
            [
                ["0"],
                ["1"],
                ["1"],
                ["0"]
            ]);
        viewModel.SubmitTruthTableEditing();

        return viewModel;
    }
}
