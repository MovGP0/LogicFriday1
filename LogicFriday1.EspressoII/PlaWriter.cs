namespace Espresso;

public static class PlaWriter
{
    private const int Dash = 3, Zero = 1;

    public static void Write(TextWriter fp, PlaData pla, int outputType)
    {
        CubeData cube = pla.Cube;
        if (outputType == PlaData.EqntottType)
        {
            if (cube.Output == -1) throw new InvalidOperationException("Cannot have no-output function for EQNTOTT output mode");
            if (cube.NumMvVars != 1) throw new InvalidOperationException("Must have binary-valued function for EQNTOTT output mode");
            pla.Label ??= new string?[cube.Size];
            for (int var = 0; var < cube.NumVars; var++)
            for (int ei = 0; ei < cube.PartSize[var]; ei++)
            {
                int ind = cube.FirstPart[var] + ei;
                pla.Label[ind] ??= var < cube.NumBinaryVars
                    ? (ei % 2 == 0 ? $"v{var}.bar" : $"v{var}")
                    : $"v{var}.{ei}";
            }
            for (int i = 0; i < cube.PartSize[cube.Output]; i++)
            {
                string ol = GetLabel(pla, cube, cube.Output, i);
                fp.Write($"{ol} = ");
                int col = ol.Length + 3;
                bool firstOr = true;
                for (int si = 0; si < pla.F!.Count; si++)
                {
                    ReadOnlySpan<uint> sp = pla.F.GetSpan(si);
                    if (!BitVectorOps.Contains(sp, i + cube.FirstPart[cube.Output])) continue;
                    if (firstOr) { fp.Write('('); col++; }
                    else { fp.Write(" | ("); col += 4; }
                    firstOr = false;
                    bool firstAnd = true;
                    for (int var = 0; var < cube.NumBinaryVars; var++)
                    {
                        int x = GetInput(sp, var);
                        if (x == Dash) continue;
                        string il = GetLabel(pla, cube, var, 1);
                        if (col + il.Length > 72) { fp.Write("\n    "); col = 4; }
                        if (!firstAnd) { fp.Write('&'); col++; }
                        firstAnd = false;
                        if (x == Zero) { fp.Write('!'); col++; }
                        fp.Write(il); col += il.Length;
                    }
                    fp.Write(')'); col++;
                }
                fp.Write(";\n\n");
            }
        }
        else if (outputType == PlaData.FType)
        {
            if (cube.NumMvVars <= 1)
            {
                fp.WriteLine($".i {cube.NumBinaryVars}");
                if (cube.Output != -1) fp.WriteLine($".o {cube.PartSize[cube.Output]}");
            }
            else
            {
                fp.Write($".mv {cube.NumVars} {cube.NumBinaryVars}");
                for (int var = cube.NumBinaryVars; var < cube.NumVars; var++) fp.Write($" {cube.PartSize[var]}");
                fp.Write('\n');
            }
            if (pla.Label != null && cube.NumBinaryVars > 0 && pla.Label[1] != null)
            {
                fp.Write(".ilb");
                for (int var = 0; var < cube.NumBinaryVars; var++) fp.Write($" {GetLabel(pla, cube, var, 1)}");
                fp.Write('\n');
            }
            if (pla.Label != null && cube.Output != -1 && pla.Label[cube.FirstPart[cube.Output]] != null)
            {
                fp.Write(".ob");
                for (int i = 0; i < cube.PartSize[cube.Output]; i++) fp.Write($" {GetLabel(pla, cube, cube.Output, i)}");
                fp.Write('\n');
            }
            for (int var = cube.NumBinaryVars; var < cube.NumVars - 1; var++)
            {
                if (pla.Label != null && pla.Label[cube.FirstPart[var]] != null)
                {
                    fp.Write($".label var={var}");
                    for (int i = cube.FirstPart[var]; i <= cube.LastPart[var]; i++) fp.Write($" {pla.Label[i]}");
                    fp.Write('\n');
                }
            }
            fp.WriteLine($".p {pla.F!.Count}");
            for (int i = 0; i < pla.F.Count; i++)
            {
                BitVector c = pla.F.GetSet(i);
                ReadOnlySpan<uint> sc = c.AsSpan();
                for (int var = 0; var < cube.NumBinaryVars; var++) fp.Write("?01-"[GetInput(sc, var)]);
                for (int var = cube.NumBinaryVars; var < cube.NumVars - 1; var++)
                {
                    fp.Write(' ');
                    for (int j = cube.FirstPart[var]; j <= cube.LastPart[var]; j++) fp.Write("01"[BitVectorOps.Contains(sc, j) ? 1 : 0]);
                }
                if (cube.Output != -1)
                {
                    fp.Write(' ');
                    for (int j = cube.FirstPart[cube.Output]; j <= cube.LastPart[cube.Output]; j++) fp.Write("01"[BitVectorOps.Contains(sc, j) ? 1 : 0]);
                }
                fp.Write('\n');
            }
            fp.WriteLine(".e");
        }
        else throw new InvalidOperationException("Unsupported output type");
    }

    private static int GetInput(ReadOnlySpan<uint> c, int var) =>
        (int)((c[BitVectorOps.WhichWord(2 * var)] >> BitVectorOps.WhichBit(2 * var)) & 3u);

    private static string GetLabel(PlaData pla, CubeData cube, int var, int offset) =>
        pla.Label![cube.FirstPart[var] + offset]!;

}