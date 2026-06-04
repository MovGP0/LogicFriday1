namespace LogicFriday1.Services;

public sealed record LookupFunctionGenerationResult(
    string FileName,
    string FileExtension,
    string FileTypeName,
    string SourceCode);
