using LogicFriday1.ViewModels;
using Shouldly;
using Xunit;

namespace LogicFriday1.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public void TruthTableShowMode_TogglesReadOnlyFunctionRows()
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

        viewModel.IsTruthTableShowModeEnabled.ShouldBeTrue();
        viewModel.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue();
        GetTerms(viewModel).ShouldBe(["1", "2", "3"]);

        viewModel.ShowAllTruthTableRows();

        viewModel.IsShowAllTruthTableRowsSelected.ShouldBeTrue();
        GetTerms(viewModel).ShouldBe(["0", "1", "2", "3"]);

        viewModel.ShowTrueAndDontCareTruthTableRows();

        viewModel.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue();
        GetTerms(viewModel).ShouldBe(["1", "2", "3"]);
    }

    [Fact]
    public void TruthTableShowMode_IsTrackedPerFunction()
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

        viewModel.IsShowTrueAndDontCareTruthTableRowsSelected.ShouldBeTrue();
        GetTerms(viewModel).ShouldBe(["1"]);

        viewModel.SelectedFunctionSummary = firstSummary;
        viewModel.ShowFunction(firstSummary!.LogicFunction!);

        viewModel.IsShowAllTruthTableRowsSelected.ShouldBeTrue();
        GetTerms(viewModel).ShouldBe(["0", "1"]);
    }

    private static string[] GetTerms(MainWindowViewModel viewModel)
    {
        return viewModel.FunctionTruthTableRows
            .Select(static row => row.Cells[0].Value)
            .ToArray();
    }
}
