namespace Espresso;

public class SparseEntry
{
    public int Key;
    public SortedIntArray Refs = new();
    public int RowNum { get => Key; set => Key = value; }
    public SortedIntArray Cols => Refs;
}