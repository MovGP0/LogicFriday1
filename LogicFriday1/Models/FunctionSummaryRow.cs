namespace LogicFriday1.Models;

public sealed record FunctionSummaryRow(
    string Function = "",
    string Inputs = "",
    string Outputs = "",
    string True = "",
    string False = "",
    string DC = "",
    string PI = "",
    string Gates = "");
