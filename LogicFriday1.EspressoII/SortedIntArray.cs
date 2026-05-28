namespace Espresso;

public sealed class SortedIntArray
{
    private int[] _items;

    private int _count;

    public SortedIntArray()
    {
        _items = [];
    }

    public SortedIntArray(SortedIntArray other)
    {
        _count = other._count;
        _items = _count > 0 ? other._items.AsSpan(0, _count).ToArray() : [];
    }

    public static SortedIntArray FromSorted(ReadOnlySpan<int> sorted)
    {
        var a = new SortedIntArray { _items = sorted.ToArray(), _count = sorted.Length };
        return a;
    }

    public int Count => _count;

    public int Min => _items[0];

    public int Max => _items[_count - 1];

    public void Add(int value)
    {
        // Monotone-append fast path (common when inserting columns in order)
        if (_count > 0 && value > _items[_count - 1])
        {
            if (_count == _items.Length)
            {
                var n = new int[Math.Max(_items.Length * 2, 4)];
                Array.Copy(_items, n, _count);
                _items = n;
            }

            _items[_count++] = value;
            return;
        }

        int pos = Array.BinarySearch(_items, 0, _count, value);
        if (pos >= 0)
        {
            return;
        }

        pos = ~pos;
        if (_count == _items.Length)
        {
            var n = new int[Math.Max(_items.Length * 2, 4)];
            if (_count > 0)
            {
                Array.Copy(_items, n, _count);
            }

            _items = n;
        }

        if (pos < _count)
        {
            Array.Copy(_items, pos, _items, pos + 1, _count - pos);
        }

        _items[pos] = value;
        _count++;
    }

    public void Remove(int value)
    {
        int pos = Array.BinarySearch(_items, 0, _count, value);
        if (pos < 0)
        {
            return;
        }

        _count--;
        if (pos < _count)
        {
            Array.Copy(_items, pos + 1, _items, pos, _count - pos);
        }
    }

    public bool Overlaps(SortedIntArray other)
    {
        int ai = 0, bi = 0, ac = _count, bc = other._count;
        var a = _items;
        var b = other._items;
        while (ai < ac && bi < bc)
        {
            if (a[ai] < b[bi])
            {
                ai++;
            }
            else if (a[ai] > b[bi])
            {
                bi++;
            }
            else
            {
                return true;
            }
        }

        return false;
    }

    public bool IsSupersetOf(SortedIntArray other)
    {
        int ac = _count, bc = other._count;
        if (ac < bc)
        {
            return false;
        }

        int ai = 0, bi = 0;
        var a = _items;
        var b = other._items;
        while (bi < bc)
        {
            if (ai >= ac)
            {
                return false;
            }

            if (a[ai] < b[bi])
            {
                ai++;
            }
            else if (a[ai] == b[bi])
            {
                ai++;
                bi++;
            }
            else
            {
                return false;
            }
        }

        return true;
    }

    public void CopyTo(int[] array, int index) => Array.Copy(_items, 0, array, index, _count);

    public Enumerator GetEnumerator() => new(_items, _count);

    public struct Enumerator
    {
        private readonly int[] _items;

        private readonly int _count;

        private int _index;

        internal Enumerator(int[] items, int count)
        {
            _items = items;
            _count = count;
            _index = -1;
        }

        public int Current => _items[_index];

        public bool MoveNext() => ++_index < _count;
    }
}
