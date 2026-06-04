using System.Text;
using LogicFriday1.Models;

namespace LogicFriday1.Services;

public static class GateDiagramPackageInfoService
{
    public static string BuildPackageInfo(GateDiagramFunction gateDiagramFunction)
    {
        var rows = gateDiagramFunction.Items
            .Where(static item => TryGetPackage(item, out _, out _))
            .GroupBy(static item =>
            {
                TryGetPackage(item, out var packageName, out var capacity);
                return new PackageKey(packageName, capacity);
            })
            .Select(static group => new PackageRow(
                group.Key.Name,
                (int)Math.Ceiling(group.Count() / (double)group.Key.Capacity)))
            .Where(static row => row.Quantity > 0)
            .OrderBy(static row => row.Name, StringComparer.Ordinal)
            .ToArray();

        var builder = new StringBuilder();
        builder.AppendLine("IC\tQty");
        foreach (var row in rows)
        {
            builder.Append(row.Name);
            builder.Append('\t');
            builder.AppendLine(row.Quantity.ToString());
        }

        builder.Append("TOTAL PACKAGES\t");
        builder.Append(rows.Sum(static row => row.Quantity));
        return builder.ToString();
    }

    private static bool TryGetPackage(
        GateDiagramItem item,
        out string packageName,
        out int capacity)
    {
        (packageName, capacity) = item.Kind switch
        {
            GatePaletteKind.Not => ("Hex Inverter", 6),
            GatePaletteKind.Xor => ("Quad 2-Input EXOR", 4),
            GatePaletteKind.Mux => ("Quad 2-Input MUX", 4),
            GatePaletteKind.Nand => ($"{GetInputPackagePrefix(item.InputCount)} NAND", GetInputPackageCapacity(item.InputCount)),
            GatePaletteKind.Nor => ($"{GetInputPackagePrefix(item.InputCount)} NOR", GetInputPackageCapacity(item.InputCount)),
            GatePaletteKind.And => ($"{GetInputPackagePrefix(item.InputCount)} AND", GetInputPackageCapacity(item.InputCount)),
            GatePaletteKind.Or => ($"{GetInputPackagePrefix(item.InputCount)} OR", GetInputPackageCapacity(item.InputCount)),
            _ => ("", 0)
        };

        return capacity > 0;
    }

    private static string GetInputPackagePrefix(int inputCount)
    {
        return inputCount switch
        {
            2 => "Quad 2-Input",
            3 => "Triple 3-Input",
            4 => "Dual 4-Input",
            _ => $"{inputCount}-Input"
        };
    }

    private static int GetInputPackageCapacity(int inputCount)
    {
        return inputCount switch
        {
            2 => 4,
            3 => 3,
            4 => 2,
            _ => 1
        };
    }

    private sealed record PackageKey(
        string Name,
        int Capacity);

    private sealed record PackageRow(
        string Name,
        int Quantity);
}
