using LogicFriday1.Models;
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
    public void OperationCloneCompareEnablement_IsDisabledWithoutSelection()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsOperationCloneFunctionEnabled.ShouldBeFalse(),
            static vm => vm.IsOperationCompareFunctionsEnabled.ShouldBeFalse());
    }

    [Fact]
    public void OperationCloneCompareEnablement_EnablesCloneForSingleSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();

        AddEquation(viewModel, "F = A;");

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsOperationCloneFunctionEnabled.ShouldBeTrue(),
            static vm => vm.IsOperationCompareFunctionsEnabled.ShouldBeFalse());
    }

    [Fact]
    public void OperationCloneCompareEnablement_EnablesCompareForTwoSelectedFunctions()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        viewModel.SetSelectedFunctionCount(2);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsOperationCloneFunctionEnabled.ShouldBeFalse(),
            static vm => vm.IsOperationCompareFunctionsEnabled.ShouldBeTrue());
    }

    [Fact]
    public void CloneSelectedFunction_DuplicatesSelectedFunction()
    {
        var viewModel = CreateXorTruthTableFunction();
        var originalSummary = viewModel.SelectedFunctionSummary;
        var originalFunction = viewModel.GetSelectedFunction()!;

        var result = viewModel.CloneSelectedFunction();

        var clonedFunction = viewModel.GetSelectedFunction()!;
        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            vm => vm.FunctionSummaries.Count.ShouldBe(2),
            vm => vm.FunctionSummaries[0].ShouldBeSameAs(originalSummary),
            vm => vm.FunctionSummaries[0].LogicFunction.ShouldBeSameAs(originalFunction),
            vm => vm.SelectedFunctionSummary.ShouldNotBeSameAs(originalSummary),
            _ => clonedFunction.ShouldNotBeSameAs(originalFunction),
            _ => clonedFunction.EquationText.ShouldBe(originalFunction.EquationText),
            _ => clonedFunction.InputNames.ShouldNotBeSameAs(originalFunction.InputNames),
            _ => clonedFunction.OutputNames.ShouldNotBeSameAs(originalFunction.OutputNames),
            _ => clonedFunction.OutputValues.ShouldNotBeSameAs(originalFunction.OutputValues),
            _ => clonedFunction.OutputValues[0].ShouldNotBeSameAs(originalFunction.OutputValues[0]),
            static vm => vm.StatusText.ShouldBe("Function cloned"));
    }

    [Fact]
    public void CompareSelectedFunctions_ReportsUnavailableSelection()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");
        viewModel.SetSelectedFunctionCount(2);

        var result = viewModel.CompareSelectedFunctions();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Select two functions to compare"));
    }

    [Fact]
    public void CompareSelectedFunctions_ReportsEquivalentFunctions()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A"],
            ["0", "1"],
            ["0", "1"]);

        var result = viewModel.CompareSelectedFunctions();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            static vm => vm.StatusText.ShouldBe("Functions are equivalent"));
    }

    [Fact]
    public void CompareSelectedFunctions_ReportsDifferentFunctions()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A"],
            ["0", "1"],
            ["1", "0"]);

        var result = viewModel.CompareSelectedFunctions();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            static vm => vm.StatusText.ShouldBe("Functions are different"));
    }

    [Fact]
    public void TwoFunctionOperationEnablement_IsEnabledForTwoSelectedFunctions()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A"],
            ["0", "1"],
            ["1", "0"]);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsOperationTwoFunctionEnabled.ShouldBeTrue(),
            static vm => vm.IsOperationOrFunctionsEnabled.ShouldBeTrue(),
            static vm => vm.IsOperationAndFunctionsEnabled.ShouldBeTrue(),
            static vm => vm.IsOperationXorFunctionsEnabled.ShouldBeTrue());
    }

    [Fact]
    public void TwoFunctionOperationEnablement_IsDisabledForOneSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();

        AddTruthTableFunction(viewModel, ["A"], "F", ["0", "1"]);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsOperationTwoFunctionEnabled.ShouldBeFalse(),
            static vm => vm.IsOperationOrFunctionsEnabled.ShouldBeFalse(),
            static vm => vm.IsOperationAndFunctionsEnabled.ShouldBeFalse(),
            static vm => vm.IsOperationXorFunctionsEnabled.ShouldBeFalse());
    }

    [Fact]
    public void OrSelectedFunctions_CreatesTruthTableWithDontCareRules()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A", "B"],
            ["0", "1", "X", "X"],
            ["0", "0", "1", "X"]);

        viewModel.OrSelectedFunctions(out var errorMessage);

        viewModel.ShouldSatisfyAllConditions(
            static vm => GetSelectedOutputValues(vm).ShouldBe(["0", "1", "1", "X"]),
            static vm => vm.GetSelectedFunction()!.OutputNames.ShouldBe(["F_OR_G"]),
            static vm => vm.StatusText.ShouldBe("OR Functions created"),
            _ => errorMessage.ShouldBeNull());
    }

    [Fact]
    public void AndSelectedFunctions_CreatesTruthTableWithDontCareRules()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A", "B"],
            ["0", "1", "X", "X"],
            ["0", "1", "0", "X"]);

        viewModel.AndSelectedFunctions(out var errorMessage);

        viewModel.ShouldSatisfyAllConditions(
            static vm => GetSelectedOutputValues(vm).ShouldBe(["0", "1", "0", "X"]),
            static vm => vm.GetSelectedFunction()!.OutputNames.ShouldBe(["F_AND_G"]),
            static vm => vm.StatusText.ShouldBe("AND Functions created"),
            _ => errorMessage.ShouldBeNull());
    }

    [Fact]
    public void XorSelectedFunctions_CreatesTruthTableWithDontCareRules()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A", "B"],
            ["0", "1", "X", "X"],
            ["0", "0", "1", "X"]);

        viewModel.XorSelectedFunctions(out var errorMessage);

        viewModel.ShouldSatisfyAllConditions(
            static vm => GetSelectedOutputValues(vm).ShouldBe(["0", "1", "X", "X"]),
            static vm => vm.GetSelectedFunction()!.OutputNames.ShouldBe(["F_XOR_G"]),
            static vm => vm.StatusText.ShouldBe("XOR Functions created"),
            _ => errorMessage.ShouldBeNull());
    }

    [Fact]
    public void TwoFunctionOperation_FailsWhenInputNamesDiffer()
    {
        var viewModel = new MainWindowViewModel();
        var firstSummary = AddTruthTableFunction(viewModel, ["A"], "F", ["0", "1"]);
        var secondSummary = AddTruthTableFunction(viewModel, ["B"], "G", ["0", "1"]);

        viewModel.SetSelectedFunctionSummaries([firstSummary, secondSummary]);
        var result = viewModel.OrSelectedFunctions(out var errorMessage);

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeFalse(),
            _ => errorMessage.ShouldBe("Functions must have the same inputs and exactly one output."),
            static vm => vm.StatusText.ShouldBe("Functions must have the same inputs and exactly one output."),
            static vm => vm.FunctionSummaries.Count.ShouldBe(2));
    }

    [Fact]
    public void OperationCancelEnablement_IsDisabledWithoutActiveOperation()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.IsOperationCancelEnabled.ShouldBeFalse();
    }

    [Fact]
    public void CancelOperation_ReportsNoActiveOperation()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.CancelOperation();

        viewModel.StatusText.ShouldBe("No operation is active");
    }

    [Fact]
    public void TruthTableSubmitCommandEnablement_IsDisabledOutsideTruthTableEntryMode()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsTruthTableSubmitEnabled.ShouldBeFalse(),
            static vm => vm.IsTruthTableCancelEnabled.ShouldBeFalse());
    }

    [Fact]
    public void TruthTableSubmitCommandEnablement_IsEnabledDuringTruthTableEntryMode()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewTruthTable(["A"], ["F"]);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsTruthTableSubmitEnabled.ShouldBeTrue(),
            static vm => vm.IsTruthTableCancelEnabled.ShouldBeTrue());
    }

    [Fact]
    public void TruthTableSubmitCommandEnablement_IsDisabledAfterTruthTableSubmit()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewTruthTable(["A"], ["F"]);
        viewModel.SubmitTruthTableEditing();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsTruthTableSubmitEnabled.ShouldBeFalse(),
            static vm => vm.IsTruthTableCancelEnabled.ShouldBeFalse());
    }

    [Fact]
    public void TruthTableSubmitCommandEnablement_IsDisabledAfterTruthTableCancel()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewTruthTable(["A"], ["F"]);
        viewModel.CancelTruthTableEditing();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsTruthTableSubmitEnabled.ShouldBeFalse(),
            static vm => vm.IsTruthTableCancelEnabled.ShouldBeFalse());
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

    private static string[] GetSelectedOutputValues(MainWindowViewModel viewModel)
    {
        return viewModel.GetSelectedFunction()!
            .OutputValues
            .Select(static row => row[0])
            .ToArray();
    }

    private static void AddEquation(MainWindowViewModel viewModel, string equationText)
    {
        viewModel.StartNewLogicEquation();
        viewModel.LogicEquationText = equationText;
        viewModel.SubmitLogicEquationEditing();
    }

    private static FunctionSummaryRow AddTruthTableFunction(
        MainWindowViewModel viewModel,
        string[] inputNames,
        string outputName,
        string[] outputValues)
    {
        viewModel.StartImportedTruthTable(
            inputNames,
            [outputName],
            outputValues
                .Select(static value => new[]
                {
                    value
                })
                .ToArray());
        viewModel.SubmitTruthTableEditing();

        return viewModel.SelectedFunctionSummary!;
    }

    private static MainWindowViewModel CreateTwoSelectedTruthTableFunctions(
        string[] inputNames,
        string[] firstOutputValues,
        string[] secondOutputValues)
    {
        var viewModel = new MainWindowViewModel();
        var firstSummary = AddTruthTableFunction(viewModel, inputNames, "F", firstOutputValues);
        var secondSummary = AddTruthTableFunction(viewModel, inputNames, "G", secondOutputValues);
        viewModel.SetSelectedFunctionSummaries([firstSummary, secondSummary]);

        return viewModel;
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
