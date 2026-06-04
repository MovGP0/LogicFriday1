using LogicFriday1.Models;

namespace LogicFriday1.Services;

public interface ILogicFunctionFileService
{
    LogicFunction Load(string filePath);

    void Save(string filePath, LogicFunction logicFunction);
}
