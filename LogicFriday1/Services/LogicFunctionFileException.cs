namespace LogicFriday1.Services;

public sealed class LogicFunctionFileException : Exception
{
    public LogicFunctionFileException(string message)
        : base(message)
    {
    }

    public LogicFunctionFileException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
