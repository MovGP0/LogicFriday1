namespace Espresso;

public sealed class SparseDict(int capacity = 16)
{
    private SparseEntry?[] _items = new SparseEntry?[Math.Max(capacity, 4)];

    private int _count;

    public int Count => _count;

    public int Capacity => _items.Length;

    public SparseEntry? this[int key] => (uint)key < (uint)_items.Length ? _items[key] : null;

    public bool TryGetValue(int key, [System.Diagnostics.CodeAnalysis.NotNullWhen(true)] out SparseEntry? entry)
    {
        if ((uint)key < (uint)_items.Length)
        {
            entry = _items[key];
            return entry != null;
        }

        entry = null;
        return false;
    }

    public void Set(int key, SparseEntry entry)
    {
        EnsureCapacity(key + 1);
        if (_items[key] == null)
        {
            _count++;
        }

        _items[key] = entry;
    }

    public bool Remove(int key)
    {
        if ((uint)key >= (uint)_items.Length || _items[key] == null)
        {
            return false;
        }

        _items[key] = null;
        _count--;
        return true;
    }

    public int CopyKeysTo(int[] buf)
    {
        int idx = 0;
        for (int i = 0; i < _items.Length && idx < _count; i++)
        {
            if (_items[i] != null)
            {
                buf[idx++] = i;
            }
        }

        return idx;
    }

    private void EnsureCapacity(int needed)
    {
        if (needed <= _items.Length)
        {
            return;
        }

        int newSize = _items.Length;
        while (newSize < needed)
        {
            newSize *= 2;
        }

        Array.Resize(ref _items, newSize);
    }

    public Enumerator GetEnumerator() => new(_items, _count);

    public struct Enumerator
    {
        private readonly SparseEntry?[] _items;

        private readonly int _total;

        private int _index;

        private int _seen;

        internal Enumerator(SparseEntry?[] items, int count)
        {
            _items = items;
            _total = count;
            _index = -1;
            _seen = 0;
        }

        public SparseEntry Current => _items[_index]!;

        public bool MoveNext()
        {
            if (_seen >= _total)
            {
                return false;
            }

            while (++_index < _items.Length)
            {
                if (_items[_index] != null)
                {
                    _seen++;
                    return true;
                }
            }

            return false;
        }
    }
}
