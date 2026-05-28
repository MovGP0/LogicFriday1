namespace Espresso;

public readonly struct SplitStack
{
    private readonly uint[] _data;

    private readonly int _words;

    private readonly int _stride;

    public SplitStack(int size, int maxDepth)
    {
        _words = BitVectorOps.WordCount(size);
        _stride = _words + 1;
        _data = new uint[maxDepth * 2 * _stride];
    }

    public void GetPair(int depth, out BitVector cl, out BitVector cr)
    {
        int offset = depth * 2 * _stride;
        cl = new BitVector(_data, offset + 1, _words);
        cr = new BitVector(_data, offset + _stride + 1, _words);
    }
}
