using LogicFriday1.Models;
using LogicFriday1.Services;
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
    public void FileCommandEnablement_AllowsNewAndOpenWithoutFunctions()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileNewEnabled.ShouldBeTrue(),
            static vm => vm.IsFileOpenEnabled.ShouldBeTrue(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeFalse(),
            static vm => vm.IsFileExportEnabled.ShouldBeFalse(),
            static vm => vm.IsFilePrintEnabled.ShouldBeFalse());
    }

    [Fact]
    public void FileCommandEnablement_AllowsSaveAsExportAndPrintForSingleSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();

        AddEquation(viewModel, "F = A;");

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileNewEnabled.ShouldBeTrue(),
            static vm => vm.IsFileOpenEnabled.ShouldBeTrue(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeTrue(),
            static vm => vm.IsFileExportEnabled.ShouldBeTrue(),
            static vm => vm.IsFilePrintEnabled.ShouldBeTrue());
    }

    [Fact]
    public void FileCommandEnablement_DisablesExportAndPrintForMultipleSelectedFunctions()
    {
        var viewModel = CreateTwoSelectedTruthTableFunctions(
            ["A"],
            ["0", "1"],
            ["1", "0"]);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileNewEnabled.ShouldBeTrue(),
            static vm => vm.IsFileOpenEnabled.ShouldBeTrue(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeFalse(),
            static vm => vm.IsFileExportEnabled.ShouldBeFalse(),
            static vm => vm.IsFilePrintEnabled.ShouldBeFalse());
    }

    [Fact]
    public void FileCommandEnablement_DisablesSharedFileCommandsDuringCreationMode()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewLogicEquation();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileNewEnabled.ShouldBeFalse(),
            static vm => vm.IsFileOpenEnabled.ShouldBeFalse(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeFalse(),
            static vm => vm.IsFileExportEnabled.ShouldBeFalse(),
            static vm => vm.IsFilePrintEnabled.ShouldBeFalse());
    }

    [Fact]
    public void PrintSelectedFunction_ReportsUnsupportedStatusText()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        var result = viewModel.PrintSelectedFunction();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            static vm => vm.StatusText.ShouldBe("Print is not yet supported in this port"));
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
    public void FilePersistenceEnablement_IsDisabledWithoutSelection()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileOpenEnabled.ShouldBeTrue(),
            static vm => vm.IsFileSaveEnabled.ShouldBeFalse(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeFalse(),
            static vm => vm.SelectedFunctionFilePath.ShouldBeNull(),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeFalse());
    }

    [Fact]
    public void FilePersistenceEnablement_IsEnabledForSingleSelectedFunction()
    {
        var viewModel = new MainWindowViewModel();

        AddEquation(viewModel, "F = A;");

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileSaveEnabled.ShouldBeTrue(),
            static vm => vm.IsFileSaveAsEnabled.ShouldBeTrue(),
            static vm => vm.SelectedFunctionFilePath.ShouldBeNull(),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeTrue());
    }

    [Fact]
    public void SaveSelectedFunction_WithoutPath_ReportsSaveAsRequired()
    {
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        var result = viewModel.SaveSelectedFunction();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Save As required"),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeTrue());
    }

    [Fact]
    public void SaveSelectedFunctionAs_SavesPathAndMarksFunctionClean()
    {
        using var tempFiles = new MainWindowViewModelPersistenceTempFiles();
        var viewModel = new MainWindowViewModel();
        AddEquation(viewModel, "F = A;");

        var result = viewModel.SaveSelectedFunctionAs(tempFiles.GetFilePath("saved.lfcn"));

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            vm => File.Exists(vm.SelectedFunctionFilePath).ShouldBeTrue(),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Function saved"));
    }

    [Fact]
    public void SaveSelectedFunction_WithExistingPath_SavesCurrentFunctionAndMarksClean()
    {
        using var tempFiles = new MainWindowViewModelPersistenceTempFiles();
        var viewModel = new MainWindowViewModel();
        var filePath = tempFiles.GetFilePath("saved.lfcn");
        AddEquation(viewModel, "F = A;");
        viewModel.SaveSelectedFunctionAs(filePath);

        viewModel.ShowSumOfProductsEquation();
        var saveResult = viewModel.SaveSelectedFunction();
        var loadedFunction = new LogicFunctionFileService().Load(filePath);

        viewModel.ShouldSatisfyAllConditions(
            _ => saveResult.ShouldBeTrue(),
            static vm => vm.SelectedFunctionFilePath.ShouldNotBeNull(),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeFalse(),
            _ => loadedFunction.EquationText.ShouldBe(viewModel.GetSelectedFunction()!.EquationText));
    }

    [Fact]
    public void OpenFunction_LoadsFunctionAndMarksCleanWithPath()
    {
        using var tempFiles = new MainWindowViewModelPersistenceTempFiles();
        var filePath = tempFiles.GetFilePath("open.lfcn");
        new LogicFunctionFileService().Save(
            filePath,
            new TruthTableLogicFunction(
                ["A"],
                ["F"],
                [
                    ["0"],
                    ["1"]
                ],
                "F = A;"));
        var viewModel = new MainWindowViewModel();

        var result = viewModel.OpenFunction(filePath);

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            static vm => vm.GetSelectedFunction()!.OutputNames.ShouldBe(["F"]),
            vm => vm.SelectedFunctionFilePath.ShouldBe(filePath),
            static vm => vm.IsSelectedFunctionDirty.ShouldBeFalse(),
            static vm => vm.StatusText.ShouldBe("Function opened"));
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

    [Fact]
    public void FileExportEnablement_IsEnabledForSingleSelectedFunction()
    {
        var viewModel = CreateXorTruthTableFunction();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileExportEnabled.ShouldBeTrue(),
            static vm => vm.IsFileExportTruthTableEnabled.ShouldBeTrue(),
            static vm => vm.IsFileExportGateDiagramEnabled.ShouldBeFalse());
    }

    [Fact]
    public void FileExportEnablement_EnablesGateDiagramForGateDiagramFunctionWithItems()
    {
        var viewModel = new MainWindowViewModel();
        var gateDiagramFunction = CreateGateDiagramFunction();
        var summary = new FunctionSummaryRow(
            Function: "F",
            Inputs: "1",
            Outputs: "1",
            Gates: "0",
            LogicFunction: gateDiagramFunction);

        viewModel.FunctionSummaries.Clear();
        viewModel.FunctionSummaries.Add(summary);
        viewModel.SelectedFunctionSummary = summary;
        viewModel.SelectedFunctionCount = 1;

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsFileExportEnabled.ShouldBeTrue(),
            static vm => vm.IsFileExportTruthTableEnabled.ShouldBeTrue(),
            static vm => vm.IsFileExportGateDiagramEnabled.ShouldBeTrue());
    }

    [Fact]
    public void ExportSelectedTruthTableCsv_ReturnsCsvForSelectedFunction()
    {
        var viewModel = CreateXorTruthTableFunction();

        var csv = viewModel.ExportSelectedTruthTableCsv();

        viewModel.ShouldSatisfyAllConditions(
            _ => csv.ShouldBe("A,B,,F\r\n0,0,,0\r\n0,1,,1\r\n1,0,,1\r\n1,1,,0\r\n"),
            static vm => vm.StatusText.ShouldBe("Truth table exported"));
    }

    [Fact]
    public void ExportSelectedGateDiagramSvg_ReturnsSvgForSelectedGateDiagram()
    {
        var viewModel = new MainWindowViewModel();
        var gateDiagramFunction = CreateGateDiagramFunction();
        var summary = new FunctionSummaryRow(
            Function: "F",
            Inputs: "1",
            Outputs: "1",
            Gates: "0",
            LogicFunction: gateDiagramFunction);

        viewModel.FunctionSummaries.Clear();
        viewModel.FunctionSummaries.Add(summary);
        viewModel.SelectedFunctionSummary = summary;
        viewModel.SelectedFunctionCount = 1;

        var svg = viewModel.ExportSelectedGateDiagramSvg();

        viewModel.ShouldSatisfyAllConditions(
            _ => svg.ShouldNotBeNull(),
            _ => svg!.ShouldContain("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            static vm => vm.StatusText.ShouldBe("Gate diagram exported"));
    }

    [Fact]
    public void GatesMenuEnablement_IsEnabledForSingleSelectedGateDiagramWithItems()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsGatesModifyGateDiagramEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesCopyToClipboardEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesIcPackageInfoEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicEnabled.ShouldBeTrue());
    }

    [Fact]
    public void GatesMenuEnablement_RemainsEnabledWhenSelectedGateDiagramIsDisplayed()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        viewModel.ShowFunction(viewModel.GetSelectedFunction()!);

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsGateDiagramVisible.ShouldBeTrue(),
            static vm => vm.IsGatesModifyGateDiagramEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesCopyToClipboardEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesIcPackageInfoEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicEnabled.ShouldBeTrue());
    }

    [Fact]
    public void GatesMenuEnablement_IsDisabledForMultipleSelectedFunctions()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        viewModel.SelectedFunctionCount = 2;

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsGatesModifyGateDiagramEnabled.ShouldBeFalse(),
            static vm => vm.IsGatesCopyToClipboardEnabled.ShouldBeFalse(),
            static vm => vm.IsGatesIcPackageInfoEnabled.ShouldBeFalse(),
            static vm => vm.IsGatesTraceLogicEnabled.ShouldBeFalse());
    }

    [Fact]
    public void GatesCopyToClipboardEnablement_IsEnabledForActiveEditorContent()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewGateDiagram();
        viewModel.GateDiagramItems.Add(new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1));

        viewModel.IsGatesCopyToClipboardEnabled.ShouldBeTrue();
    }

    [Fact]
    public void StartModifySelectedGateDiagram_LoadsSelectedDiagramForEditing()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        var wasStarted = viewModel.StartModifySelectedGateDiagram();

        viewModel.ShouldSatisfyAllConditions(
            _ => wasStarted.ShouldBeTrue(),
            static vm => vm.IsGateDiagramVisible.ShouldBeTrue(),
            static vm => vm.GateDiagramItems.Select(static item => item.Label).ShouldBe(["A", "F"]),
            static vm => vm.GateDiagramWires.Count.ShouldBe(1),
            static vm => vm.StatusText.ShouldBe("Modifying gate diagram"));
    }

    [Fact]
    public void SubmitGateDiagramEditing_ReplacesModifiedGateDiagram()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateAndGateDiagramFunction());

        viewModel.StartModifySelectedGateDiagram();
        viewModel.GateDiagramItems[0] = viewModel.GateDiagramItems[0] with
        {
            X = 24
        };
        var wasSubmitted = viewModel.SubmitGateDiagramEditing(out var errorMessage);

        viewModel.ShouldSatisfyAllConditions(
            _ => wasSubmitted.ShouldBeTrue(),
            _ => errorMessage.ShouldBeNull(),
            static vm => vm.FunctionSummaries.Count.ShouldBe(1),
            static vm => ((GateDiagramFunction)vm.GetSelectedFunction()!).Items[0].X.ShouldBe(24),
            static vm => vm.StatusText.ShouldBe("Gate diagram modified"));
    }

    [Fact]
    public void CreateGateDiagramClipboardSvg_ReturnsSvgForSelectedGateDiagram()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        var svg = viewModel.CreateGateDiagramClipboardSvg();

        viewModel.ShouldSatisfyAllConditions(
            _ => svg.ShouldNotBeNull(),
            _ => svg!.ShouldContain("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            static vm => vm.StatusText.ShouldBe("Gate diagram copied to clipboard"));
    }

    [Fact]
    public void CreateGateDiagramClipboardSvg_ReturnsSvgForActiveEditorContent()
    {
        var viewModel = new MainWindowViewModel();

        viewModel.StartNewGateDiagram();
        viewModel.GateDiagramItems.Add(new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1));
        var svg = viewModel.CreateGateDiagramClipboardSvg();

        viewModel.ShouldSatisfyAllConditions(
            _ => svg.ShouldNotBeNull(),
            _ => svg!.ShouldContain(">A<"),
            static vm => vm.StatusText.ShouldBe("Gate diagram copied to clipboard"));
    }

    [Fact]
    public void GetSelectedGatePackageInfo_ReturnsPackageSummaryForSelectedGateDiagram()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateAndGateDiagramFunction());

        var packageInfo = viewModel.GetSelectedGatePackageInfo();

        viewModel.ShouldSatisfyAllConditions(
            _ => packageInfo.ShouldNotBeNull(),
            _ => packageInfo!.ShouldContain("Quad 2-Input AND\t1"),
            static vm => vm.StatusText.ShouldBe("IC package information generated"));
    }

    [Fact]
    public void ToggleGateTraceLogic_EnablesTraceForSelectedGateDiagram()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateAndGateDiagramFunction());

        var wasEnabled = viewModel.ToggleGateTraceLogic();

        viewModel.ShouldSatisfyAllConditions(
            _ => wasEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicChecked.ShouldBeTrue(),
            static vm => vm.GateTraceInputs.Select(static input => input.DisplayText).ShouldBe(["A = 0", "B = 0"]),
            static vm => vm.GateTraceOutputText.ShouldBe("F = 0"),
            static vm => vm.StatusText.ShouldBe("Gate logic trace enabled"));
    }

    [Fact]
    public void SetGateTraceInputValue_RecomputesTraceOutput()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateAndGateDiagramFunction());

        viewModel.ToggleGateTraceLogic();
        viewModel.SetGateTraceInputValue("A", 1);
        var wasSet = viewModel.SetGateTraceInputValue("B", 1);

        viewModel.ShouldSatisfyAllConditions(
            _ => wasSet.ShouldBeTrue(),
            static vm => vm.GateTraceOutputText.ShouldBe("F = 1"),
            static vm => vm.StatusText.ShouldBe("Gate logic trace recomputed"));
    }

    [Fact]
    public void GatesPackageInfoAndTraceEnablement_IsDisabledWithoutSelectedGateDiagram()
    {
        var viewModel = CreateXorTruthTableFunction();

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsGatesIcPackageInfoEnabled.ShouldBeFalse(),
            static vm => vm.IsGatesTraceLogicEnabled.ShouldBeFalse(),
            static vm => vm.IsGatesTraceLogicChecked.ShouldBeFalse());
    }

    [Fact]
    public void GatesPackageInfoAndTraceEnablement_IsEnabledForSelectedGateDiagram()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateAndGateDiagramFunction());

        viewModel.ShouldSatisfyAllConditions(
            static vm => vm.IsGatesIcPackageInfoEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicEnabled.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicChecked.ShouldBeFalse());
    }

    [Fact]
    public void ToggleGateTraceLogic_DisablesActiveTrace()
    {
        var viewModel = CreateSelectedGateDiagramViewModel(CreateGateDiagramFunction());

        viewModel.ToggleGateTraceLogic();
        var result = viewModel.ToggleGateTraceLogic();

        viewModel.ShouldSatisfyAllConditions(
            _ => result.ShouldBeTrue(),
            static vm => vm.IsGatesTraceLogicChecked.ShouldBeFalse(),
            static vm => vm.GateTraceInputs.ShouldBeEmpty(),
            static vm => vm.GateTraceOutputText.ShouldBe(""),
            static vm => vm.StatusText.ShouldBe("Gate logic trace disabled"));
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

    private static GateDiagramFunction CreateGateDiagramFunction()
    {
        return new GateDiagramFunction(
            ["A"],
            ["F"],
            [
                ["0"],
                ["1"]
            ],
            "F = A;",
            [
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1),
                new GateDiagramItem(GatePaletteKind.Output, 1, 120, 0, "F", Id: 2)
            ],
            [
                new GateDiagramWire(
                    new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Input, 0))
            ]);
    }

    private static GateDiagramFunction CreateAndGateDiagramFunction()
    {
        return new GateDiagramFunction(
            ["A", "B"],
            ["F"],
            [
                ["0"],
                ["0"],
                ["0"],
                ["1"]
            ],
            "F = A & B;",
            [
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 0, "A", Id: 1),
                new GateDiagramItem(GatePaletteKind.Input, 0, 0, 80, "B", Id: 2),
                new GateDiagramItem(GatePaletteKind.And, 2, 120, 40, "", Id: 3),
                new GateDiagramItem(GatePaletteKind.Output, 1, 240, 40, "F", Id: 4)
            ],
            [
                new GateDiagramWire(
                    new GateDiagramConnectionReference(1, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 0)),
                new GateDiagramWire(
                    new GateDiagramConnectionReference(2, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Input, 1)),
                new GateDiagramWire(
                    new GateDiagramConnectionReference(3, GateDiagramConnectionKind.Output, 0),
                    new GateDiagramConnectionReference(4, GateDiagramConnectionKind.Input, 0))
            ]);
    }

    private static MainWindowViewModel CreateSelectedGateDiagramViewModel(GateDiagramFunction gateDiagramFunction)
    {
        var viewModel = new MainWindowViewModel();
        var summary = new FunctionSummaryRow(
            Function: "F",
            Inputs: gateDiagramFunction.InputNames.Length.ToString(),
            Outputs: gateDiagramFunction.OutputNames.Length.ToString(),
            Gates: gateDiagramFunction.Items
                .Count(static item => item.Kind is not GatePaletteKind.Input and not GatePaletteKind.Output)
                .ToString(),
            LogicFunction: gateDiagramFunction);

        viewModel.FunctionSummaries.Clear();
        viewModel.FunctionSummaries.Add(summary);
        viewModel.SelectedFunctionSummary = summary;
        viewModel.SelectedFunctionCount = 1;

        return viewModel;
    }
}

public sealed class MainWindowViewModelPersistenceTempFiles : IDisposable
{
    private readonly string _tempDirectory = Path.Combine(
        Directory.GetCurrentDirectory(),
        ".temp",
        Guid.NewGuid().ToString("N"));

    public MainWindowViewModelPersistenceTempFiles()
    {
        Directory.CreateDirectory(_tempDirectory);
    }

    public string GetFilePath(string fileName)
    {
        return Path.Combine(_tempDirectory, fileName);
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDirectory))
        {
            Directory.Delete(_tempDirectory, recursive: true);
        }
    }
}
